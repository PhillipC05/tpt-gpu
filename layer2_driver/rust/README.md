# tptd

TPT GPU userspace driver library — Rust bindings over the TPT kernel driver ABI.

## Overview

`tptd` is the thin Rust library that wraps the ioctl ABI exposed by the TPT Linux kernel module (`tptd.ko`). It provides safe Rust types for device open/close, memory mapping, and command submission, and is the bridge between `tptr-core` and the kernel driver.

## Usage

```toml
[dependencies]
tptd = "0.1"
```

Enable the software simulation backend for CI (no hardware required):

```toml
tptd = { version = "0.1", features = ["sim"] }
```

## License

Apache-2.0 — see the [repository](https://github.com/tpt-solutions/tpt-gpu) for details.
