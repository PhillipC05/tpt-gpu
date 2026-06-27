/* SPDX-License-Identifier: Apache-2.0 */
/*
 * tpt_driver.h — TPT GPU shared driver ABI
 *
 * This header defines the ioctl interface and data structures shared between
 * the kernel driver and userspace (tptd). It is the FFI boundary consumed by
 * the Rust userspace crate via bindgen.
 */

#ifndef TPT_DRIVER_H
#define TPT_DRIVER_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

/* -------------------------------------------------------------------------
 * Version
 * ---------------------------------------------------------------------- */
#define TPT_DRIVER_MAJOR 1
#define TPT_DRIVER_MINOR 0

/* -------------------------------------------------------------------------
 * Buffer flags
 * ---------------------------------------------------------------------- */
#define TPT_BUF_FLAG_VRAM     (1u << 0)  /* prefer VRAM placement */
#define TPT_BUF_FLAG_GTT      (1u << 1)  /* GTT / system-RAM-backed */
#define TPT_BUF_FLAG_CPU_MAP  (1u << 2)  /* will be CPU-mapped by userspace */
#define TPT_BUF_FLAG_COHERENT (1u << 3)  /* cache-coherent with CPU */

/* -------------------------------------------------------------------------
 * Memory domains (for eviction / migration hints)
 * ---------------------------------------------------------------------- */
typedef enum tpt_mem_domain {
    TPT_DOMAIN_CPU    = 0,
    TPT_DOMAIN_VRAM   = 1,
    TPT_DOMAIN_GTT    = 2,
} tpt_mem_domain_t;

/* -------------------------------------------------------------------------
 * Ioctl structures
 * ---------------------------------------------------------------------- */

/* TPT_IOCTL_GEM_CREATE — allocate a GEM buffer object */
struct tpt_gem_create {
    uint64_t size;      /* in: allocation size in bytes (page-aligned) */
    uint32_t flags;     /* in: TPT_BUF_FLAG_* */
    uint32_t handle;    /* out: GEM handle */
};

/* TPT_IOCTL_GEM_FREE — release a GEM buffer object */
struct tpt_gem_free {
    uint32_t handle;    /* in: GEM handle to release */
    uint32_t _pad;
};

/* TPT_IOCTL_GEM_INFO — query buffer object properties */
struct tpt_gem_info {
    uint32_t handle;    /* in */
    uint32_t _pad;
    uint64_t size;      /* out: actual size in bytes */
    uint64_t gpu_addr;  /* out: GPU virtual address (0 if not mapped to GPU) */
};

/* TPT_IOCTL_GEM_MMAP — prepare buffer for CPU mmap (returns fake offset) */
struct tpt_gem_mmap {
    uint32_t handle;    /* in */
    uint32_t _pad;
    uint64_t offset;    /* out: mmap offset for use with mmap(2) */
};

/* TPT_IOCTL_SUBMIT — submit a command buffer for execution */
struct tpt_submit {
    uint32_t cmd_handle;    /* in: GEM handle of command buffer */
    uint32_t cmd_offset;    /* in: byte offset into command buffer */
    uint32_t cmd_size;      /* in: command buffer size in bytes */
    uint32_t flags;         /* in: submission flags (reserved, must be 0) */
    uint64_t fence_seqno;   /* out: timeline seqno for this submission */
};

/* TPT_IOCTL_WAIT_FENCE — block until a submission completes */
struct tpt_wait_fence {
    uint64_t fence_seqno;   /* in: seqno returned by TPT_IOCTL_SUBMIT */
    uint64_t timeout_ns;    /* in: timeout in nanoseconds (UINT64_MAX = infinite) */
};

/* TPT_IOCTL_QUERY_INFO — query device capabilities */
struct tpt_query_info {
    uint32_t query;         /* in: TPT_QUERY_* */
    uint32_t _pad;
    uint64_t value;         /* out */
};

#define TPT_QUERY_VRAM_SIZE     0x01  /* total VRAM in bytes */
#define TPT_QUERY_VRAM_FREE     0x02  /* free VRAM in bytes */
#define TPT_QUERY_NUM_WARPS     0x03  /* hardware warp pool size */
#define TPT_QUERY_NUM_CTAS      0x04  /* max concurrent CTAs */
#define TPT_QUERY_DRIVER_VER    0x05  /* (major << 16) | minor */
#define TPT_QUERY_WARP_LANES    0x06  /* lanes per warp */

/* -------------------------------------------------------------------------
 * Ioctl numbers  (Linux _IOWR convention; Windows/macOS wrappers use same IDs)
 * ---------------------------------------------------------------------- */
#ifdef __linux__
#include <linux/ioctl.h>
#define TPT_IOCTL_BASE          'T'
#define TPT_IOCTL_GEM_CREATE    _IOWR(TPT_IOCTL_BASE, 0x01, struct tpt_gem_create)
#define TPT_IOCTL_GEM_FREE      _IOW (TPT_IOCTL_BASE, 0x02, struct tpt_gem_free)
#define TPT_IOCTL_GEM_INFO      _IOWR(TPT_IOCTL_BASE, 0x03, struct tpt_gem_info)
#define TPT_IOCTL_GEM_MMAP      _IOWR(TPT_IOCTL_BASE, 0x04, struct tpt_gem_mmap)
#define TPT_IOCTL_SUBMIT        _IOWR(TPT_IOCTL_BASE, 0x05, struct tpt_submit)
#define TPT_IOCTL_WAIT_FENCE    _IOW (TPT_IOCTL_BASE, 0x06, struct tpt_wait_fence)
#define TPT_IOCTL_QUERY_INFO    _IOWR(TPT_IOCTL_BASE, 0x07, struct tpt_query_info)
#else
/* Windows / macOS use ioctl ID literals directly */
#define TPT_IOCTL_GEM_CREATE    0x01
#define TPT_IOCTL_GEM_FREE      0x02
#define TPT_IOCTL_GEM_INFO      0x03
#define TPT_IOCTL_GEM_MMAP      0x04
#define TPT_IOCTL_SUBMIT        0x05
#define TPT_IOCTL_WAIT_FENCE    0x06
#define TPT_IOCTL_QUERY_INFO    0x07
#endif /* __linux__ */

/* -------------------------------------------------------------------------
 * Userspace-facing C API (implemented in tptd, not the kernel module)
 * ---------------------------------------------------------------------- */
typedef struct tpt_device  tpt_device_t;
typedef struct tpt_buffer  tpt_buffer_t;
typedef struct tpt_fence   tpt_fence_t;

/* Opaque handle returned by tpt_open() */
tpt_device_t *tpt_open(const char *device_path);
void          tpt_close(tpt_device_t *dev);

tpt_buffer_t *tpt_buffer_alloc(tpt_device_t *dev, uint64_t size, uint32_t flags);
void          tpt_buffer_free(tpt_buffer_t *buf);
void         *tpt_buffer_map(tpt_buffer_t *buf);
void          tpt_buffer_unmap(tpt_buffer_t *buf);
uint64_t      tpt_buffer_gpu_addr(tpt_buffer_t *buf);

tpt_fence_t  *tpt_submit(tpt_device_t *dev, tpt_buffer_t *cmdbuf,
                          uint32_t offset, uint32_t size);
int           tpt_fence_wait(tpt_fence_t *fence, uint64_t timeout_ns);
void          tpt_fence_free(tpt_fence_t *fence);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* TPT_DRIVER_H */
