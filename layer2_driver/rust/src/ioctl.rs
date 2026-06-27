// ioctl.rs — raw Linux ioctl wrappers for the TPT GPU driver ABI.
//
// Structs must exactly match the kernel-side layout in tpt_driver.h.
// All fields are native-endian; the kernel driver does no byte-swapping.

use crate::{Result, TptError};
use libc::c_int;

// ---------------------------------------------------------------------------
// Ioctl number construction (Linux _IOWR style)
// ---------------------------------------------------------------------------
const IOC_NRBITS:   u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
const IOC_NRSHIFT:  u32 = 0;
const IOC_TYPESHIFT:u32 = IOC_NRSHIFT   + IOC_NRBITS;
const IOC_SIZESHIFT:u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;
const IOC_WRITE:    u32 = 1;
const IOC_READ:     u32 = 2;

const fn _ioc(dir: u32, ty: u8, nr: u8, size: usize) -> u32 {
    (dir << IOC_DIRSHIFT)
        | ((ty as u32) << IOC_TYPESHIFT)
        | ((nr as u32) << IOC_NRSHIFT)
        | ((size as u32) << IOC_SIZESHIFT)
}
const fn _iowr(ty: u8, nr: u8, size: usize) -> u32 { _ioc(IOC_READ | IOC_WRITE, ty, nr, size) }
const fn _iow (ty: u8, nr: u8, size: usize) -> u32 { _ioc(IOC_WRITE, ty, nr, size) }

const TPT_IOCTL_BASE: u8 = b'T';

// ---------------------------------------------------------------------------
// Kernel ABI structs (repr(C) + packed to match kernel layout)
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct GemCreate {
    pub size:   u64,
    pub flags:  u32,
    pub handle: u32,
}

#[repr(C)]
pub struct GemFree {
    pub handle: u32,
    pub _pad:   u32,
}

#[repr(C)]
pub struct GemInfo {
    pub handle:   u32,
    pub _pad:     u32,
    pub size:     u64,
    pub gpu_addr: u64,
}

#[repr(C)]
pub struct GemMmap {
    pub handle: u32,
    pub _pad:   u32,
    pub offset: u64,
}

#[repr(C)]
pub struct Submit {
    pub cmd_handle:  u32,
    pub cmd_offset:  u32,
    pub cmd_size:    u32,
    pub flags:       u32,
    pub fence_seqno: u64,
}

#[repr(C)]
pub struct WaitFence {
    pub fence_seqno: u64,
    pub timeout_ns:  u64,
}

#[repr(C)]
pub struct QueryInfo {
    pub query: u32,
    pub _pad:  u32,
    pub value: u64,
}

// ---------------------------------------------------------------------------
// Ioctl codes
// ---------------------------------------------------------------------------
const IOCTL_GEM_CREATE:  u32 = _iowr(TPT_IOCTL_BASE, 0x01, std::mem::size_of::<GemCreate>());
const IOCTL_GEM_FREE:    u32 = _iow (TPT_IOCTL_BASE, 0x02, std::mem::size_of::<GemFree>());
const IOCTL_GEM_INFO:    u32 = _iowr(TPT_IOCTL_BASE, 0x03, std::mem::size_of::<GemInfo>());
const IOCTL_GEM_MMAP:    u32 = _iowr(TPT_IOCTL_BASE, 0x04, std::mem::size_of::<GemMmap>());
const IOCTL_SUBMIT:      u32 = _iowr(TPT_IOCTL_BASE, 0x05, std::mem::size_of::<Submit>());
const IOCTL_WAIT_FENCE:  u32 = _iow (TPT_IOCTL_BASE, 0x06, std::mem::size_of::<WaitFence>());
const IOCTL_QUERY_INFO:  u32 = _iowr(TPT_IOCTL_BASE, 0x07, std::mem::size_of::<QueryInfo>());

// ---------------------------------------------------------------------------
// Safe wrappers
// ---------------------------------------------------------------------------
fn ioctl<T>(fd: c_int, request: u32, arg: &mut T) -> Result<()> {
    let ret = unsafe { libc::ioctl(fd, request as libc::Ioctl, arg as *mut T) };
    if ret == -1 {
        Err(TptError::Ioctl(std::io::Error::last_os_error().to_string()))
    } else {
        Ok(())
    }
}

pub fn gem_create(fd: c_int, size: u64, flags: u32) -> Result<u32> {
    let mut args = GemCreate { size, flags, handle: 0 };
    ioctl(fd, IOCTL_GEM_CREATE, &mut args)?;
    Ok(args.handle)
}

pub fn gem_free(fd: c_int, handle: u32) -> Result<()> {
    let mut args = GemFree { handle, _pad: 0 };
    ioctl(fd, IOCTL_GEM_FREE, &mut args)
}

pub fn gem_info(fd: c_int, handle: u32) -> Result<(u64, u64)> {
    let mut args = GemInfo { handle, _pad: 0, size: 0, gpu_addr: 0 };
    ioctl(fd, IOCTL_GEM_INFO, &mut args)?;
    Ok((args.size, args.gpu_addr))
}

pub fn gem_mmap(fd: c_int, handle: u32) -> Result<u64> {
    let mut args = GemMmap { handle, _pad: 0, offset: 0 };
    ioctl(fd, IOCTL_GEM_MMAP, &mut args)?;
    Ok(args.offset)
}

pub fn submit(fd: c_int, cmd_handle: u32, cmd_offset: u32, cmd_size: u32) -> Result<u64> {
    let mut args = Submit {
        cmd_handle,
        cmd_offset,
        cmd_size,
        flags: 0,
        fence_seqno: 0,
    };
    ioctl(fd, IOCTL_SUBMIT, &mut args)?;
    Ok(args.fence_seqno)
}

pub fn wait_fence(fd: c_int, fence_seqno: u64, timeout_ns: u64) -> Result<()> {
    let mut args = WaitFence { fence_seqno, timeout_ns };
    ioctl(fd, IOCTL_WAIT_FENCE, &mut args)
}

pub fn query_info(fd: c_int, query: u32) -> Result<u64> {
    let mut args = QueryInfo { query, _pad: 0, value: 0 };
    ioctl(fd, IOCTL_QUERY_INFO, &mut args)?;
    Ok(args.value)
}
