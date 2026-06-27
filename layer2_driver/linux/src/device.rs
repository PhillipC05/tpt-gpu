// SPDX-License-Identifier: Apache-2.0
//
// device.rs — TPT GPU device state, command submission, and query ioctls.

use kernel::prelude::*;
use kernel::{drm, pci, sync::{Arc, Mutex, SpinLock}, time};

use crate::regs;

// ---------------------------------------------------------------------------
// Device state
// ---------------------------------------------------------------------------
pub struct TptDevice {
    pub(crate) drm:    drm::device::Device<crate::TptDrmDriver>,
    bar0:              pci::IoMap,
    inner:             Mutex<DeviceInner>,
    fence_queue:       kernel::wait::WaitQueue,
}

struct DeviceInner {
    next_seqno:     u64,
    completed_seqno: u64,
    error:          bool,
}

impl TptDevice {
    pub fn new(
        pdev: &pci::Device,
        drm: drm::device::Device<crate::TptDrmDriver>,
    ) -> Result<Self> {
        let bar0 = pdev.iomap_region(0, c_str!("tpt_gpu_regs"), regs::BAR0_SIZE)?;

        // Enable interrupt mask for fence + error.
        let irq_mask = regs::IRQ_FENCE_SIGNALED | regs::IRQ_ERROR;
        bar0.writel(irq_mask, regs::REG_IRQ_MASK as usize);

        // Enable warp scheduler.
        bar0.writel(1, regs::REG_SCHED_ENABLE as usize);

        Ok(Self {
            drm,
            bar0,
            inner: Mutex::new(DeviceInner {
                next_seqno: 1,
                completed_seqno: 0,
                error: false,
            }),
            fence_queue: kernel::wait::WaitQueue::new(),
        })
    }

    pub fn read_reg(&self, reg: u32) -> u32 {
        self.bar0.readl(reg as usize)
    }

    pub fn write_reg(&self, reg: u32, val: u32) {
        self.bar0.writel(val, reg as usize);
    }

    /// Called from IRQ handler when a fence completion interrupt fires.
    pub fn fence_signaled(&self) {
        let seqno = self.read_reg(regs::REG_FENCE_SEQNO) as u64;
        {
            let mut inner = self.inner.lock();
            inner.completed_seqno = seqno;
        }
        self.fence_queue.wake_all();
    }

    pub fn set_error(&self) {
        self.inner.lock().error = true;
        self.fence_queue.wake_all();
    }

    pub fn shutdown(&self) {
        // Mask all interrupts before device is torn down.
        self.write_reg(regs::REG_IRQ_MASK, 0);
        self.write_reg(regs::REG_SCHED_ENABLE, 0);
    }
}

// ---------------------------------------------------------------------------
// IOCTL: submit command buffer
// ---------------------------------------------------------------------------
pub fn ioctl_submit(
    dev: &drm::device::Device<crate::TptDrmDriver>,
    data: &mut crate::bindings::tpt_submit,
    _file: &drm::File<()>,
) -> Result {
    let tpt = dev.get_drvdata();
    let mut inner = tpt.inner.lock();

    if inner.error {
        return Err(ENODEV);
    }

    // Validate command buffer handle and size.
    if data.cmd_size == 0 || data.cmd_size % 4 != 0 {
        return Err(EINVAL);
    }

    // Look up GEM handle → GPU address.
    let gem_obj = dev.lookup_gem_object(data.cmd_handle)?;
    let gpu_addr = gem_obj.gpu_addr();

    // Write command buffer address + size into ring.
    // Ring entry format: [CMD_ADDR_LO, CMD_ADDR_HI, CMD_SIZE, FENCE_SEQNO]
    let seqno = inner.next_seqno;
    inner.next_seqno += 1;
    drop(inner);

    tpt.write_reg(regs::REG_RING_BASE, (gpu_addr + data.cmd_offset as u64) as u32);
    tpt.write_reg(regs::REG_FENCE_EMIT, seqno as u32);
    // Advance write pointer to trigger GPU scheduling.
    let wptr = tpt.read_reg(regs::REG_RING_WPTR).wrapping_add(4);
    tpt.write_reg(regs::REG_RING_WPTR, wptr);

    data.fence_seqno = seqno;
    Ok(())
}

// ---------------------------------------------------------------------------
// IOCTL: wait for fence
// ---------------------------------------------------------------------------
pub fn ioctl_wait_fence(
    dev: &drm::device::Device<crate::TptDrmDriver>,
    data: &mut crate::bindings::tpt_wait_fence,
    _file: &drm::File<()>,
) -> Result {
    let tpt = dev.get_drvdata();
    let seqno = data.fence_seqno;
    let timeout_ns = data.timeout_ns;

    let deadline = if timeout_ns == u64::MAX {
        None
    } else {
        Some(time::ktime_get() + timeout_ns as i64)
    };

    tpt.fence_queue.wait_event_timeout(
        || {
            let inner = tpt.inner.lock();
            if inner.error {
                return Err(ENODEV);
            }
            if inner.completed_seqno >= seqno {
                Ok(true)
            } else {
                Ok(false)
            }
        },
        deadline,
    )?;

    Ok(())
}

// ---------------------------------------------------------------------------
// IOCTL: query device info
// ---------------------------------------------------------------------------
pub fn ioctl_query_info(
    dev: &drm::device::Device<crate::TptDrmDriver>,
    data: &mut crate::bindings::tpt_query_info,
    _file: &drm::File<()>,
) -> Result {
    use crate::bindings::*;
    let tpt = dev.get_drvdata();

    data.value = match data.query {
        TPT_QUERY_VRAM_SIZE  => tpt.read_reg(regs::REG_VRAM_SIZE) as u64,
        TPT_QUERY_VRAM_FREE  => tpt.read_reg(regs::REG_VRAM_FREE) as u64,
        TPT_QUERY_NUM_WARPS  => tpt.read_reg(regs::REG_NUM_WARPS) as u64,
        TPT_QUERY_NUM_CTAS   => tpt.read_reg(regs::REG_NUM_CTAS) as u64,
        TPT_QUERY_WARP_LANES => tpt.read_reg(regs::REG_WARP_LANES) as u64,
        TPT_QUERY_DRIVER_VER =>
            ((crate::bindings::TPT_DRIVER_MAJOR as u64) << 16)
            | (crate::bindings::TPT_DRIVER_MINOR as u64),
        _ => return Err(EINVAL),
    };

    Ok(())
}
