// submit.rs — command buffer building and GPU submission.
//
// `CmdBuf` is a typed builder that writes TPT ISA command packets into a
// GPU-accessible buffer.  Calling `Device::submit` hands it to the kernel
// driver which enqueues it on the hardware ring.
//
// `Fence` wraps the returned seqno and provides blocking wait.

use std::{sync::Arc, time::Duration};
use crate::{ioctl, mem::{Buffer, BufferFlags}, DeviceFd, Result, TptError};

// ---------------------------------------------------------------------------
// Command packet opcodes (host-side encoding, matches the ISA ring protocol)
// ---------------------------------------------------------------------------
#[repr(u32)]
enum Pkt {
    Nop    = 0x0000_0000,
    Launch = 0x0100_0000, // launch a compute kernel
    Fence  = 0x0200_0000, // emit fence seqno to memory
}

// ---------------------------------------------------------------------------
// CmdBuf — command buffer builder
// ---------------------------------------------------------------------------
pub struct CmdBuf {
    pub(crate) buf:    Buffer,
    write_pos:         usize,   // byte offset into the buffer
}

impl CmdBuf {
    /// Allocate a command buffer backed by a GTT (CPU-accessible) buffer.
    pub fn new(fd: Arc<DeviceFd>, capacity: u64) -> Result<Self> {
        let flags = BufferFlags::GTT | BufferFlags::CPU_MAP | BufferFlags::COHERENT;
        let mut buf = crate::mem::alloc(fd, capacity, flags)?;
        buf.map()?;
        Ok(Self { buf, write_pos: 0 })
    }

    pub fn handle(&self)    -> u32 { self.buf.handle() }
    pub fn offset(&self)    -> u32 { 0 }
    pub fn used_bytes(&self)-> u32 { self.write_pos as u32 }

    /// Append a NOP packet.
    pub fn nop(&mut self) -> Result<&mut Self> {
        self.write_dword(Pkt::Nop as u32)
    }

    /// Append a kernel launch packet.
    ///
    /// `kernel_addr` — GPU virtual address of the ISA kernel entry point.
    /// `grid`        — (x, y, z) CTA grid dimensions.
    /// `block`       — (x, y, z) thread-block dimensions (must be ≤ WARP_LANES × N).
    pub fn launch(
        &mut self,
        kernel_addr: u64,
        grid: (u32, u32, u32),
        block: (u32, u32, u32),
    ) -> Result<&mut Self> {
        self.write_dword(Pkt::Launch as u32)?;
        self.write_dword((kernel_addr & 0xFFFF_FFFF) as u32)?; // addr lo
        self.write_dword((kernel_addr >> 32) as u32)?;          // addr hi
        self.write_dword(grid.0)?;
        self.write_dword(grid.1)?;
        self.write_dword(grid.2)?;
        self.write_dword(block.0)?;
        self.write_dword(block.1)?;
        self.write_dword(block.2)
    }

    /// Append a fence packet (GPU writes seqno to `fence_addr` when done).
    pub fn fence(&mut self, fence_addr: u64, seqno: u32) -> Result<&mut Self> {
        self.write_dword(Pkt::Fence as u32)?;
        self.write_dword((fence_addr & 0xFFFF_FFFF) as u32)?;
        self.write_dword((fence_addr >> 32) as u32)?;
        self.write_dword(seqno)
    }

    fn write_dword(&mut self, val: u32) -> Result<&mut Self> {
        let end = self.write_pos + 4;
        if end > self.buf.size() as usize {
            return Err(TptError::OutOfMemory);
        }
        let slice = self.buf.map()?;
        slice[self.write_pos..end].copy_from_slice(&val.to_le_bytes());
        self.write_pos = end;
        Ok(self)
    }

    /// Reset write pointer (reuse buffer without reallocation).
    pub fn reset(&mut self) {
        self.write_pos = 0;
    }
}

// ---------------------------------------------------------------------------
// Fence
// ---------------------------------------------------------------------------
pub struct Fence {
    fd:    Arc<DeviceFd>,
    seqno: u64,
}

impl Fence {
    pub fn seqno(&self) -> u64 { self.seqno }

    /// Block until the GPU completes this submission, or timeout elapses.
    pub fn wait(&self, timeout: Duration) -> Result<()> {
        let timeout_ns = timeout.as_nanos().min(u64::MAX as u128) as u64;
        match ioctl::wait_fence(self.fd.raw(), self.seqno, timeout_ns) {
            Err(TptError::Ioctl(ref s)) if s.contains("timed out") => Err(TptError::Timeout),
            other => other,
        }
    }

    /// Block indefinitely until the GPU completes.
    pub fn wait_forever(&self) -> Result<()> {
        ioctl::wait_fence(self.fd.raw(), self.seqno, u64::MAX)
    }
}

// ---------------------------------------------------------------------------
// Submission entry point (called via Device::submit)
// ---------------------------------------------------------------------------
pub fn submit(fd: Arc<DeviceFd>, cmdbuf: &CmdBuf) -> Result<Fence> {
    let seqno = ioctl::submit(
        fd.raw(),
        cmdbuf.handle(),
        cmdbuf.offset(),
        cmdbuf.used_bytes(),
    )?;
    Ok(Fence { fd, seqno })
}
