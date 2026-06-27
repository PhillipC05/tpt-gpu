/*
 * tpt_wdm.h — TPT GPU Windows WDM driver internal types
 *
 * Tested against WDK 10.0.22621.0 (Windows 11 22H2 SDK).
 * Build with: msbuild tpt_gpu.vcxproj /p:Configuration=Release;Platform=x64
 */

#pragma once

#include <ntddk.h>
#include <wdm.h>
#include <initguid.h>

/* Device interface GUID — {1A2E0001-0000-0000-0000-000000000001} */
DEFINE_GUID(GUID_TPT_GPU_INTERFACE,
    0x1a2e0001, 0x0000, 0x0000,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01);

/* PCI IDs */
#define TPT_PCI_VENDOR_ID  0x1A2E
#define TPT_PCI_DEVICE_ID  0x0001

/* BAR indices */
#define TPT_BAR0_INDEX     0   /* register space, 64 KiB */
#define TPT_BAR2_INDEX     2   /* VRAM aperture */

/* Register offsets (mirror of regs.rs) */
#define REG_DEVICE_ID      0x0000
#define REG_STATUS         0x0008
#define REG_RESET          0x000C
#define REG_VRAM_SIZE      0x0020
#define REG_VRAM_FREE      0x0024
#define REG_RING_WPTR      0x0048
#define REG_FENCE_SEQNO    0x0060
#define REG_FENCE_EMIT     0x0064
#define REG_IRQ_STATUS     0x0080
#define REG_IRQ_MASK       0x0084
#define REG_IRQ_ACK        0x0088
#define REG_SCHED_ENABLE   0x0100
#define REG_NUM_WARPS      0x0104
#define REG_NUM_CTAS       0x0108
#define REG_WARP_LANES     0x010C

#define STATUS_READY       0x00000001
#define STATUS_ERROR       0x00000002

#define IRQ_FENCE_SIGNALED 0x00000001
#define IRQ_ERROR          0x80000000

/* Pool tag for ExAllocatePoolWithTag */
#define TPT_POOL_TAG       'tptG'

/* Buffer flags (kept in sync with tpt_driver.h) */
#define TPT_BUF_FLAG_VRAM      (1u << 0)
#define TPT_BUF_FLAG_GTT       (1u << 1)
#define TPT_BUF_FLAG_CPU_MAP   (1u << 2)
#define TPT_BUF_FLAG_COHERENT  (1u << 3)

/* -------------------------------------------------------------------------
 * GEM-equivalent buffer descriptor
 * ---------------------------------------------------------------------- */
typedef struct _TPT_BUFFER {
    LIST_ENTRY      ListEntry;
    ULONG           Handle;
    SIZE_T          Size;
    UINT32          Flags;
    UINT64          GpuAddress;
    PMDL            Mdl;            /* MDL for user-space mapping */
    PVOID           KernelVa;      /* kernel virtual mapping */
} TPT_BUFFER, *PTPT_BUFFER;

/* -------------------------------------------------------------------------
 * Per-device extension (attached to PDO by AddDevice)
 * ---------------------------------------------------------------------- */
typedef struct _TPT_DEVICE_EXT {
    /* PCI state */
    PDEVICE_OBJECT      PhysicalDeviceObject;
    PDEVICE_OBJECT      NextLowerDevice;
    BUS_INTERFACE_STANDARD BusInterface;

    /* MMIO mappings */
    PVOID               Bar0Va;         /* mapped BAR0 register space */
    PHYSICAL_ADDRESS    Bar0Pa;
    ULONG               Bar0Size;

    PVOID               Bar2Va;         /* mapped BAR2 VRAM aperture */
    PHYSICAL_ADDRESS    Bar2Pa;
    ULONGLONG           Bar2Size;

    /* Interrupt */
    PKINTERRUPT         InterruptObject;
    KDPC                FenceCompleteDpc;

    /* Buffer table (protected by BufferLock) */
    KSPIN_LOCK          BufferLock;
    LIST_ENTRY          BufferList;
    ULONG               NextHandle;

    /* Fence / synchronization */
    KSPIN_LOCK          FenceLock;
    ULONGLONG           CompletedSeqno;
    ULONGLONG           NextSeqno;
    KEVENT              FenceEvent;

    /* Device interface */
    UNICODE_STRING      SymbolicLinkName;
    BOOLEAN             InterfaceRegistered;

    /* Error state */
    BOOLEAN             HardwareError;
} TPT_DEVICE_EXT, *PTPT_DEVICE_EXT;

/* -------------------------------------------------------------------------
 * IOCTL codes (DeviceIoControl)
 * ---------------------------------------------------------------------- */
#define TPT_DEVICE_TYPE     0x8A2E

#define IOCTL_TPT_GEM_CREATE \
    CTL_CODE(TPT_DEVICE_TYPE, 0x801, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_TPT_GEM_FREE \
    CTL_CODE(TPT_DEVICE_TYPE, 0x802, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_TPT_GEM_INFO \
    CTL_CODE(TPT_DEVICE_TYPE, 0x803, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_TPT_SUBMIT \
    CTL_CODE(TPT_DEVICE_TYPE, 0x804, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_TPT_WAIT_FENCE \
    CTL_CODE(TPT_DEVICE_TYPE, 0x805, METHOD_BUFFERED, FILE_ANY_ACCESS)
#define IOCTL_TPT_QUERY_INFO \
    CTL_CODE(TPT_DEVICE_TYPE, 0x806, METHOD_BUFFERED, FILE_ANY_ACCESS)

/* Shared ioctl structs (same layout as tpt_driver.h) */
#include "../../include/tpt_driver.h"

/* Internal helpers */
ULONG     TptReadReg32(PTPT_DEVICE_EXT ext, ULONG offset);
VOID      TptWriteReg32(PTPT_DEVICE_EXT ext, ULONG offset, ULONG value);
NTSTATUS  TptAllocateBuffer(PTPT_DEVICE_EXT ext, UINT64 size, UINT32 flags,
                             PULONG outHandle);
NTSTATUS  TptFreeBuffer(PTPT_DEVICE_EXT ext, ULONG handle);
PTPT_BUFFER TptLookupBuffer(PTPT_DEVICE_EXT ext, ULONG handle);
