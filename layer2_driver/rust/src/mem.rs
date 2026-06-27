// mem.rs — GPU buffer allocation, CPU mapping, and lifetime management.
//
// `Buffer` owns a GEM handle and, optionally, a CPU mmap.
// Dropping a `Buffer` unmaps and frees it automatically.

use std::{
    os::raw::c_int,
    ptr, slice,
    sync::Arc,
};
use crate::{ioctl, DeviceFd, Result, TptError};

// ---------------------------------------------------------------------------
// Buffer flags (mirror tpt_driver.h)
// ---------------------------------------------------------------------------
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct BufferFlags: u32 {
        const VRAM     = 1 << 0;
        const GTT      = 1 << 1;
        const CPU_MAP  = 1 << 2;
        const COHERENT = 1 << 3;
    }
}

// ---------------------------------------------------------------------------
// Buffer
// ---------------------------------------------------------------------------
pub struct Buffer {
    fd:       Arc<DeviceFd>,
    handle:   u32,
    size:     u64,
    gpu_addr: u64,
    cpu_ptr:  *mut u8,   // null if not mapped
}

// SAFETY: Buffer holds a pointer that is only ever accessed via &mut self
// or after checking `is_mapped()`. The fd Arc makes it Send.
unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

impl Buffer {
    pub fn handle(&self)   -> u32   { self.handle }
    pub fn size(&self)     -> u64   { self.size }
    pub fn gpu_addr(&self) -> u64   { self.gpu_addr }
    pub fn is_mapped(&self) -> bool { !self.cpu_ptr.is_null() }

    /// Map the buffer into CPU address space.
    ///
    /// The pointer is valid for the lifetime of `self`.
    pub fn map(&mut self) -> Result<&mut [u8]> {
        if self.is_mapped() {
            return Ok(self.cpu_slice_mut());
        }
        let offset = ioctl::gem_mmap(self.fd.raw(), self.handle)?;
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                self.size as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.fd.raw(),
                offset as libc::off_t,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(TptError::Io(std::io::Error::last_os_error()));
        }
        self.cpu_ptr = ptr as *mut u8;
        Ok(self.cpu_slice_mut())
    }

    /// Unmap the CPU mapping (no-op if not mapped).
    pub fn unmap(&mut self) {
        if self.is_mapped() {
            unsafe { libc::munmap(self.cpu_ptr as *mut libc::c_void, self.size as libc::size_t) };
            self.cpu_ptr = ptr::null_mut();
        }
    }

    fn cpu_slice_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.cpu_ptr, self.size as usize) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.unmap();
        let _ = ioctl::gem_free(self.fd.raw(), self.handle);
    }
}

// ---------------------------------------------------------------------------
// Allocation entry point (called via Device::alloc)
// ---------------------------------------------------------------------------
pub fn alloc(fd: Arc<DeviceFd>, size: u64, flags: BufferFlags) -> Result<Buffer> {
    if size == 0 {
        return Err(TptError::OutOfMemory);
    }
    let handle = ioctl::gem_create(fd.raw(), size, flags.bits())?;
    let (actual_size, gpu_addr) = ioctl::gem_info(fd.raw(), handle)?;
    Ok(Buffer {
        fd,
        handle,
        size: actual_size,
        gpu_addr,
        cpu_ptr: ptr::null_mut(),
    })
}
