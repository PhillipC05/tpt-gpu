// SPDX-License-Identifier: Apache-2.0
//
// regs.rs — TPT GPU MMIO register map
//
// BAR 0: control registers (64 KiB)
// BAR 2: VRAM aperture (up to 16 GiB, size reported in REG_VRAM_SIZE)

// BAR sizes for iomap_region.
pub const BAR0_SIZE: usize = 0x0001_0000; //  64 KiB
pub const BAR2_SIZE: usize = 0x4000_0000; //  1 GiB initial aperture window

// ---------------------------------------------------------------------------
// Control / status
// ---------------------------------------------------------------------------
pub const REG_DEVICE_ID:    u32 = 0x0000; // RO — vendor:device packed u32
pub const REG_FW_VERSION:   u32 = 0x0004; // RO — firmware version
pub const REG_STATUS:       u32 = 0x0008; // RO — device status flags
pub const REG_RESET:        u32 = 0x000C; // WO — write 0xDEAD to soft-reset

pub const STATUS_READY:     u32 = 1 << 0;
pub const STATUS_ERROR:     u32 = 1 << 1;
pub const STATUS_BUSY:      u32 = 1 << 2;

// ---------------------------------------------------------------------------
// Memory
// ---------------------------------------------------------------------------
pub const REG_VRAM_SIZE:    u32 = 0x0020; // RO — VRAM size in bytes
pub const REG_VRAM_FREE:    u32 = 0x0024; // RO — free VRAM in bytes
pub const REG_GTT_BASE:     u32 = 0x0028; // RW — GTT base physical address
pub const REG_GTT_SIZE:     u32 = 0x002C; // RW — GTT size in pages

// ---------------------------------------------------------------------------
// Command submission ring
// ---------------------------------------------------------------------------
pub const REG_RING_BASE:    u32 = 0x0040; // RW — ring buffer GPU address
pub const REG_RING_SIZE:    u32 = 0x0044; // RW — ring size in dwords
pub const REG_RING_WPTR:    u32 = 0x0048; // RW — write pointer (CPU writes)
pub const REG_RING_RPTR:    u32 = 0x004C; // RO — read pointer (GPU advances)

// ---------------------------------------------------------------------------
// Fence / synchronization
// ---------------------------------------------------------------------------
pub const REG_FENCE_SEQNO:  u32 = 0x0060; // RO — last completed fence seqno
pub const REG_FENCE_EMIT:   u32 = 0x0064; // WO — write seqno to emit fence packet

// ---------------------------------------------------------------------------
// Interrupt control
// ---------------------------------------------------------------------------
pub const REG_IRQ_STATUS:   u32 = 0x0080; // RO/W1C — pending interrupt bits
pub const REG_IRQ_MASK:     u32 = 0x0084; // RW — interrupt enable mask
pub const REG_IRQ_ACK:      u32 = 0x0088; // WO — acknowledge interrupts

pub const IRQ_FENCE_SIGNALED: u32 = 1 << 0;
pub const IRQ_RING_EMPTY:     u32 = 1 << 1;
pub const IRQ_ERROR:          u32 = 1 << 31;

// ---------------------------------------------------------------------------
// Warp scheduler
// ---------------------------------------------------------------------------
pub const REG_SCHED_ENABLE: u32 = 0x0100; // RW — 1 = enable warp scheduler
pub const REG_NUM_WARPS:    u32 = 0x0104; // RO — hardware warp pool size
pub const REG_NUM_CTAS:     u32 = 0x0108; // RO — max concurrent CTAs
pub const REG_WARP_LANES:   u32 = 0x010C; // RO — lanes per warp
