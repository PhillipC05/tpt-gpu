// tptd — TPT GPU userspace driver library
//
// Crate layout:
//   mem.rs     — VRAM / GTT allocator, buffer lifetime management
//   submit.rs  — command buffer builder + ioctl submission
//   ffi.rs     — extern "C" API (consumed via tpt_driver.h + bindgen)

pub mod mem;
pub mod submit;
pub mod ffi;

mod ioctl;   // raw ioctl wrappers (platform-specific, not pub)

pub use mem::{Buffer, BufferFlags};
pub use submit::{CmdBuf, Fence};

use std::{fs, os::unix::io::AsRawFd, path::Path, sync::Arc};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------
#[derive(Debug, Error)]
pub enum TptError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ioctl failed: {0}")]
    Ioctl(String),
    #[error("out of memory")]
    OutOfMemory,
    #[error("invalid handle")]
    InvalidHandle,
    #[error("timeout")]
    Timeout,
    #[error("hardware error")]
    HardwareError,
}

pub type Result<T> = std::result::Result<T, TptError>;

// ---------------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------------
pub struct Device {
    pub(crate) fd: Arc<DeviceFd>,
}

pub(crate) struct DeviceFd(std::fs::File);

impl DeviceFd {
    pub fn raw(&self) -> std::os::raw::c_int {
        self.0.as_raw_fd()
    }
}

impl Device {
    /// Open the TPT GPU device node (e.g. `/dev/dri/card0` on Linux).
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())?;
        Ok(Device {
            fd: Arc::new(DeviceFd(file)),
        })
    }

    /// Query a device property.
    pub fn query(&self, query: QueryType) -> Result<u64> {
        ioctl::query_info(self.fd.raw(), query as u32)
    }

    /// Allocate a GPU buffer.
    pub fn alloc(&self, size: u64, flags: BufferFlags) -> Result<Buffer> {
        mem::alloc(Arc::clone(&self.fd), size, flags)
    }

    /// Submit a command buffer for execution; returns a `Fence`.
    pub fn submit(&self, cmdbuf: &CmdBuf) -> Result<Fence> {
        submit::submit(Arc::clone(&self.fd), cmdbuf)
    }
}

// ---------------------------------------------------------------------------
// Query types
// ---------------------------------------------------------------------------
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum QueryType {
    VramSize    = 0x01,
    VramFree    = 0x02,
    NumWarps    = 0x03,
    NumCtas     = 0x04,
    DriverVer   = 0x05,
    WarpLanes   = 0x06,
}
