# tpt-gpu-driver

TPT GPU userspace driver library — Rust bindings over the TPT kernel driver ABI.

## Overview

`tpt-gpu-driver` is the thin Rust library that wraps the ioctl ABI exposed by the TPT Linux kernel module (`tptd.ko`). It provides safe Rust types for device open/close, memory mapping, and command submission, and is the bridge between `tpt-gpu-runtime` and the kernel driver.

## Usage

```toml
[dependencies]
tpt-gpu-driver = "0.1"
```

Enable the software simulation backend for CI (no hardware required):

```toml
tpt-gpu-driver = { version = "0.1", features = ["sim"] }
```

## License

Apache-2.0 — see the [repository](https://github.com/tpt-solutions/tpt-gpu) for details.
