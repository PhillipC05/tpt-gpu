// ffi.rs — extern "C" ABI exported by tptd as a C-compatible shared library.
//
// This is the FFI boundary consumed by:
//   - Layer 3 (tptc compiler) for buffer allocation during JIT
//   - Layer 4 (runtime) for device management and kernel launch
//   - Any C/C++ consumers via tpt_driver.h
//
// All pointers are heap-allocated Rust structs exposed as opaque handles.
// Callers must not dereference them directly.

#![allow(clippy::missing_safety_doc)]

use std::{
    ffi::CStr,
    os::raw::{c_char, c_int},
    time::Duration,
};
use crate::{Device, Buffer, BufferFlags, CmdBuf, Fence, Result, TptError};

// ---------------------------------------------------------------------------
// Error codes returned to C callers
// ---------------------------------------------------------------------------
pub const TPT_OK:            c_int =  0;
pub const TPT_ERR_IO:        c_int = -1;
pub const TPT_ERR_OOM:       c_int = -2;
pub const TPT_ERR_INVALID:   c_int = -3;
pub const TPT_ERR_TIMEOUT:   c_int = -4;
pub const TPT_ERR_HW_ERROR:  c_int = -5;

fn result_to_code(r: Result<()>) -> c_int {
    match r {
        Ok(())                       => TPT_OK,
        Err(TptError::Io(_))         => TPT_ERR_IO,
        Err(TptError::OutOfMemory)   => TPT_ERR_OOM,
        Err(TptError::Timeout)       => TPT_ERR_TIMEOUT,
        Err(TptError::HardwareError) => TPT_ERR_HW_ERROR,
        Err(_)                       => TPT_ERR_INVALID,
    }
}

// ---------------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------------

/// Open a TPT GPU device.
///
/// `path` — null-terminated path to the device node (e.g. `/dev/dri/card0`).
/// Returns an opaque `*mut Device` on success, or NULL on failure.
#[no_mangle]
pub unsafe extern "C" fn tpt_open(path: *const c_char) -> *mut Device {
    if path.is_null() { return std::ptr::null_mut(); }
    let cstr = unsafe { CStr::from_ptr(path) };
    let s = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match Device::open(s) {
        Ok(dev) => Box::into_raw(Box::new(dev)),
        Err(e)  => { log::error!("tpt_open: {e}"); std::ptr::null_mut() }
    }
}

/// Close and free a device handle.
#[no_mangle]
pub unsafe extern "C" fn tpt_close(dev: *mut Device) {
    if !dev.is_null() {
        drop(unsafe { Box::from_raw(dev) });
    }
}

/// Query a device property.  Returns the value or 0 on error.
#[no_mangle]
pub unsafe extern "C" fn tpt_query(
    dev:   *const Device,
    query: u32,
) -> u64 {
    if dev.is_null() { return 0; }
    let dev = unsafe { &*dev };
    let qt = match query {
        0x01 => crate::QueryType::VramSize,
        0x02 => crate::QueryType::VramFree,
        0x03 => crate::QueryType::NumWarps,
        0x04 => crate::QueryType::NumCtas,
        0x05 => crate::QueryType::DriverVer,
        0x06 => crate::QueryType::WarpLanes,
        _    => return 0,
    };
    dev.query(qt).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Buffer management
// ---------------------------------------------------------------------------

/// Allocate a GPU buffer.
///
/// Returns an opaque `*mut Buffer`, or NULL on failure.
#[no_mangle]
pub unsafe extern "C" fn tpt_buffer_alloc(
    dev:   *mut Device,
    size:  u64,
    flags: u32,
) -> *mut Buffer {
    if dev.is_null() { return std::ptr::null_mut(); }
    let dev = unsafe { &*dev };
    match dev.alloc(size, BufferFlags::from_bits_truncate(flags)) {
        Ok(buf)  => Box::into_raw(Box::new(buf)),
        Err(e)   => { log::error!("tpt_buffer_alloc: {e}"); std::ptr::null_mut() }
    }
}

/// Free a GPU buffer.
#[no_mangle]
pub unsafe extern "C" fn tpt_buffer_free(buf: *mut Buffer) {
    if !buf.is_null() {
        drop(unsafe { Box::from_raw(buf) });
    }
}

/// Map a buffer into CPU address space.
///
/// Returns a pointer to the first byte, or NULL on failure.
/// The pointer is valid until `tpt_buffer_unmap` or `tpt_buffer_free`.
#[no_mangle]
pub unsafe extern "C" fn tpt_buffer_map(buf: *mut Buffer) -> *mut u8 {
    if buf.is_null() { return std::ptr::null_mut(); }
    let buf = unsafe { &mut *buf };
    match buf.map() {
        Ok(slice) => slice.as_mut_ptr(),
        Err(e)    => { log::error!("tpt_buffer_map: {e}"); std::ptr::null_mut() }
    }
}

/// Unmap the CPU mapping of a buffer.
#[no_mangle]
pub unsafe extern "C" fn tpt_buffer_unmap(buf: *mut Buffer) {
    if !buf.is_null() {
        unsafe { &mut *buf }.unmap();
    }
}

/// Return the GPU virtual address of a buffer (0 if not placed on GPU yet).
#[no_mangle]
pub unsafe extern "C" fn tpt_buffer_gpu_addr(buf: *const Buffer) -> u64 {
    if buf.is_null() { return 0; }
    unsafe { &*buf }.gpu_addr()
}

/// Return the size of a buffer in bytes.
#[no_mangle]
pub unsafe extern "C" fn tpt_buffer_size(buf: *const Buffer) -> u64 {
    if buf.is_null() { return 0; }
    unsafe { &*buf }.size()
}

// ---------------------------------------------------------------------------
// Command buffer + submission
// ---------------------------------------------------------------------------

/// Allocate a command buffer of `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn tpt_cmdbuf_alloc(
    dev:      *mut Device,
    capacity: u64,
) -> *mut CmdBuf {
    if dev.is_null() { return std::ptr::null_mut(); }
    let dev = unsafe { &*dev };
    use std::sync::Arc;
    match CmdBuf::new(Arc::clone(&dev.fd), capacity) {
        Ok(cb)  => Box::into_raw(Box::new(cb)),
        Err(e)  => { log::error!("tpt_cmdbuf_alloc: {e}"); std::ptr::null_mut() }
    }
}

/// Free a command buffer.
#[no_mangle]
pub unsafe extern "C" fn tpt_cmdbuf_free(cb: *mut CmdBuf) {
    if !cb.is_null() { drop(unsafe { Box::from_raw(cb) }); }
}

/// Append a NOP packet.
#[no_mangle]
pub unsafe extern "C" fn tpt_cmdbuf_nop(cb: *mut CmdBuf) -> c_int {
    if cb.is_null() { return TPT_ERR_INVALID; }
    result_to_code(unsafe { &mut *cb }.nop().map(|_| ()))
}

/// Append a kernel launch packet.
#[no_mangle]
pub unsafe extern "C" fn tpt_cmdbuf_launch(
    cb:          *mut CmdBuf,
    kernel_addr: u64,
    grid_x: u32, grid_y: u32, grid_z: u32,
    blk_x:  u32, blk_y:  u32, blk_z:  u32,
) -> c_int {
    if cb.is_null() { return TPT_ERR_INVALID; }
    result_to_code(
        unsafe { &mut *cb }
            .launch(kernel_addr, (grid_x, grid_y, grid_z), (blk_x, blk_y, blk_z))
            .map(|_| ())
    )
}

/// Submit a command buffer to the GPU.
///
/// Returns an opaque `*mut Fence`, or NULL on failure.
#[no_mangle]
pub unsafe extern "C" fn tpt_submit(
    dev: *mut Device,
    cb:  *const CmdBuf,
) -> *mut Fence {
    if dev.is_null() || cb.is_null() { return std::ptr::null_mut(); }
    let dev = unsafe { &*dev };
    let cb  = unsafe { &*cb };
    match dev.submit(cb) {
        Ok(fence) => Box::into_raw(Box::new(fence)),
        Err(e)    => { log::error!("tpt_submit: {e}"); std::ptr::null_mut() }
    }
}

/// Wait for a fence, with a timeout in nanoseconds (u64::MAX = infinite).
#[no_mangle]
pub unsafe extern "C" fn tpt_fence_wait(
    fence:      *const Fence,
    timeout_ns: u64,
) -> c_int {
    if fence.is_null() { return TPT_ERR_INVALID; }
    let fence = unsafe { &*fence };
    let result = if timeout_ns == u64::MAX {
        fence.wait_forever()
    } else {
        fence.wait(Duration::from_nanos(timeout_ns))
    };
    result_to_code(result)
}

/// Free a fence handle.
#[no_mangle]
pub unsafe extern "C" fn tpt_fence_free(fence: *mut Fence) {
    if !fence.is_null() { drop(unsafe { Box::from_raw(fence) }); }
}

/// Return the fence sequence number.
#[no_mangle]
pub unsafe extern "C" fn tpt_fence_seqno(fence: *const Fence) -> u64 {
    if fence.is_null() { return 0; }
    unsafe { &*fence }.seqno()
}
