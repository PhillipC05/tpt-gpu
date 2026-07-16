# tptr-core

TPT Runtime — core allocator, scheduler, and kernel launch library for the TPT GPU stack.

## Overview

`tptr-core` implements the fundamental GPU runtime services used across the TPT compute platform:

- **Three-tier allocator** — Slab (fast path) → Buddy (medium) → Fallback (system)
- **Priority queue scheduler** — with aging to prevent starvation
- **Kernel launch** — `KernelConfig`, `ArgumentBuffer`, `KernelHandle`
- **LLM inference engine** — routes forward-pass ops through layer5 kernel handles; auto-detects CUDA → ROCm → Metal → TPTIR
- **KV cache** — sliding-window host-side K/V cache for indefinite-length decoding

## Usage

```toml
[dependencies]
tptr-core = "0.1"
```

## License

Apache-2.0 — see the [repository](https://github.com/tpt-solutions/tpt-gpu) for details.
