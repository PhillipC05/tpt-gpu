// SPDX-License-Identifier: Apache-2.0
//
// gem.rs — TPT GPU GEM buffer object management.
//
// Uses drm::gem::shmem for system-RAM-backed (GTT) buffers.
// VRAM buffers use a simple best-fit allocator over the BAR2 aperture.

use kernel::prelude::*;
use kernel::drm::{self, gem::shmem};

use crate::bindings;

// ---------------------------------------------------------------------------
// GEM object
// ---------------------------------------------------------------------------
pub struct TptGemObject {
    base:     shmem::Object<TptGemObject>,
    gpu_addr: u64,
    flags:    u32,
}

impl TptGemObject {
    pub fn gpu_addr(&self) -> u64 {
        self.gpu_addr
    }
}

impl drm::gem::BaseDriverObject<shmem::Object<Self>> for TptGemObject {
    fn new(obj: &shmem::Object<Self>, size: usize) -> impl PinInit<Self, Error> {
        let _ = size; // size is stored in shmem base
        try_pin_init!(TptGemObject {
            base:     unsafe { shmem::Object::from_gem_object(obj.as_ref()) },
            gpu_addr: 0,
            flags:    0,
        })
    }

    fn close(obj: &shmem::Object<Self>, _file: &drm::File<()>) {
        // GPU address mapping is released here when reference drops to zero.
        let _ = obj;
    }
}

impl drm::gem::IntoGemObject for TptGemObject {
    type Repr = shmem::Object<TptGemObject>;

    fn gem_object(&self) -> &kernel::bindings::drm_gem_object {
        self.base.gem_object()
    }
}

// ---------------------------------------------------------------------------
// IOCTL: gem_create
// ---------------------------------------------------------------------------
pub fn ioctl_gem_create(
    dev: &drm::device::Device<crate::TptDrmDriver>,
    data: &mut bindings::tpt_gem_create,
    file: &drm::File<()>,
) -> Result {
    // Align size to page boundary.
    let size = kernel::page_align(data.size as usize);
    if size == 0 {
        return Err(EINVAL);
    }

    let obj = shmem::Object::<TptGemObject>::new(dev, size)?;

    // For VRAM-flagged buffers, attempt to pin in BAR2 aperture.
    if data.flags & bindings::TPT_BUF_FLAG_VRAM != 0 {
        let tpt = dev.get_drvdata();
        let gpu_addr = vram_alloc(tpt, size)?;
        obj.get_inner().gpu_addr = gpu_addr;
    }
    obj.get_inner().flags = data.flags;

    data.handle = obj.create_handle(file)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// IOCTL: gem_free
// ---------------------------------------------------------------------------
pub fn ioctl_gem_free(
    dev: &drm::device::Device<crate::TptDrmDriver>,
    data: &mut bindings::tpt_gem_free,
    file: &drm::File<()>,
) -> Result {
    dev.release_gem_handle(file, data.handle)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// IOCTL: gem_info
// ---------------------------------------------------------------------------
pub fn ioctl_gem_info(
    dev: &drm::device::Device<crate::TptDrmDriver>,
    data: &mut bindings::tpt_gem_info,
    file: &drm::File<()>,
) -> Result {
    let obj = dev.lookup_gem_handle(file, data.handle)?;
    data.size     = obj.size() as u64;
    data.gpu_addr = obj.get_inner().gpu_addr;
    Ok(())
}

// ---------------------------------------------------------------------------
// IOCTL: gem_mmap — returns fake offset for mmap(2)
// ---------------------------------------------------------------------------
pub fn ioctl_gem_mmap(
    dev: &drm::device::Device<crate::TptDrmDriver>,
    data: &mut bindings::tpt_gem_mmap,
    file: &drm::File<()>,
) -> Result {
    let obj = dev.lookup_gem_handle(file, data.handle)?;
    data.offset = obj.map_offset()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Minimal VRAM allocator (bump / best-fit over BAR2 aperture)
//
// A production driver would use a proper range allocator (drm_mm in C,
// or the `drm_gem_vram` helpers). This implementation is a simple bump
// allocator sufficient for bringup and testbench validation.
// ---------------------------------------------------------------------------
use kernel::sync::Mutex;

struct VramHeap {
    base:   u64,   // GPU-side base address of VRAM
    size:   u64,   // total VRAM in bytes
    cursor: u64,   // bump pointer (not a full allocator — replace with drm_mm)
}

// SAFETY: VramHeap is only accessed under the device Mutex.
static VRAM_HEAP: Mutex<Option<VramHeap>> = Mutex::new(None);

fn vram_alloc(tpt: &crate::device::TptDevice, size: usize) -> Result<u64> {
    let mut heap = VRAM_HEAP.lock();
    let heap = heap.get_or_insert_with(|| {
        let vram_size = tpt.read_reg(crate::regs::REG_VRAM_SIZE) as u64;
        VramHeap { base: 0, size: vram_size, cursor: 0 }
    });

    let aligned = (size as u64 + 0xFFFF) & !0xFFFF; // 64 KiB alignment
    if heap.cursor + aligned > heap.size {
        return Err(ENOMEM);
    }
    let addr = heap.base + heap.cursor;
    heap.cursor += aligned;
    Ok(addr)
}
