// SPDX-License-Identifier: Apache-2.0
//
// tpt_drm — TPT GPU Linux DRM driver (Rust for Linux, kernel 6.1+)
//
// Entry point: module registration, PCI probe/remove, DRM device lifecycle.
// Modelled after the asahi (Apple M1) and nova (Nouveau replacement) drivers.

use kernel::prelude::*;
use kernel::{drm, pci};

mod device;
mod gem;
mod regs;

use device::TptDevice;

kernel::module! {
    type: TptModule,
    name: "tpt_gpu",
    author: "TPT GPU Contributors",
    description: "TPT GPU DRM driver",
    license: "GPL v2",
    alias: ["pci:v00001A2Ed00000001sv*sd*bc03sc00i00"],
}

// PCI device table — vendor 0x1A2E, device 0x0001 (TPT GPU prototype).
kernel::module_pci_driver! {
    type: TptDriver,
    name: "tpt_gpu",
    id_table: TPT_PCI_TABLE,
}

// Vendor / device IDs.  Update as silicon IDs are assigned.
pub const TPT_PCI_VENDOR: u32 = 0x1A2E;
pub const TPT_PCI_DEVICE: u32 = 0x0001;

static TPT_PCI_TABLE: pci::IdTable = &[
    pci::DeviceId::new(TPT_PCI_VENDOR, TPT_PCI_DEVICE),
    pci::DeviceId::new(TPT_PCI_VENDOR, 0x0002), // future TPT GPU rev B
];

// ---------------------------------------------------------------------------
// Module object (zero-size; all state lives in TptDevice per device)
// ---------------------------------------------------------------------------
struct TptModule;

impl kernel::Module for TptModule {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("TPT GPU driver loaded\n");
        Ok(TptModule)
    }
}

impl Drop for TptModule {
    fn drop(&mut self) {
        pr_info!("TPT GPU driver unloaded\n");
    }
}

// ---------------------------------------------------------------------------
// PCI driver — probe / remove
// ---------------------------------------------------------------------------
struct TptDriver;

impl pci::Driver for TptDriver {
    type Data = Arc<TptDevice>;

    fn probe(pdev: &mut pci::Device, _id: Option<&pci::DeviceId>) -> Result<Self::Data> {
        pr_info!(
            "TPT GPU: probing PCI device {:04x}:{:04x}\n",
            pdev.vendor_id(),
            pdev.device_id()
        );

        pdev.enable_device_mem()?;
        pdev.set_master();

        // Map BAR 0 (register space, 64 KiB) and BAR 2 (VRAM aperture).
        let bar0 = pdev.iomap_region(0, c_str!("tpt_gpu_regs"), regs::BAR0_SIZE)?;
        let bar2 = pdev.iomap_region(2, c_str!("tpt_gpu_vram"), regs::BAR2_SIZE)?;

        // Register DRM device.
        let drm_dev = drm::device::Device::<TptDrmDriver>::new(
            pdev.as_ref(),
            TptDeviceData { bar0, bar2 },
        )?;

        let dev = TptDevice::new(pdev, drm_dev)?;
        let dev = Arc::try_new(dev)?;

        // Request IRQ.
        pdev.request_irq(irq_handler, &dev, pci::IrqType::Msix, 1)?;

        // Advertise device to userspace.
        drm::device::Device::register(&dev.drm)?;

        pr_info!("TPT GPU: device ready\n");
        Ok(dev)
    }

    fn remove(dev: &Self::Data) {
        pr_info!("TPT GPU: removing device\n");
        dev.shutdown();
    }
}

// ---------------------------------------------------------------------------
// DRM driver vtable
// ---------------------------------------------------------------------------
struct TptDrmDriver;

struct TptDeviceData {
    bar0: pci::IoMap,
    bar2: pci::IoMap,
}

impl drm::Driver for TptDrmDriver {
    type Object = gem::TptGemObject;
    type Data = TptDeviceData;
    type File = ();
    type AuthMagic = ();

    const INFO: drm::DriverInfo = drm::DriverInfo {
        major: 1,
        minor: 0,
        patchlevel: 0,
        name: c_str!("tpt_gpu"),
        desc: c_str!("TPT GPU DRM driver"),
        date: c_str!("20260101"),
    };

    kernel::declare_drm_ioctls! {
        (TPT_IOCTL_GEM_CREATE,  tpt_gem_create,  RW, gem::ioctl_gem_create),
        (TPT_IOCTL_GEM_FREE,    tpt_gem_free,    W,  gem::ioctl_gem_free),
        (TPT_IOCTL_GEM_INFO,    tpt_gem_info,    RW, gem::ioctl_gem_info),
        (TPT_IOCTL_GEM_MMAP,    tpt_gem_mmap,    RW, gem::ioctl_gem_mmap),
        (TPT_IOCTL_SUBMIT,      tpt_submit,      RW, device::ioctl_submit),
        (TPT_IOCTL_WAIT_FENCE,  tpt_wait_fence,  W,  device::ioctl_wait_fence),
        (TPT_IOCTL_QUERY_INFO,  tpt_query_info,  RW, device::ioctl_query_info),
    }
}

// ---------------------------------------------------------------------------
// MSI-X interrupt handler
// ---------------------------------------------------------------------------
fn irq_handler(dev: &Arc<TptDevice>) -> irq::Return {
    let status = dev.read_reg(regs::REG_IRQ_STATUS);
    if status == 0 {
        return irq::Return::None;
    }
    // Acknowledge all pending interrupts.
    dev.write_reg(regs::REG_IRQ_ACK, status);

    if status & regs::IRQ_FENCE_SIGNALED != 0 {
        dev.fence_signaled();
    }
    if status & regs::IRQ_ERROR != 0 {
        pr_err!("TPT GPU: hardware error interrupt (status=0x{:08x})\n", status);
        dev.set_error();
    }

    irq::Return::Handled
}
