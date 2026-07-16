# tptr-dispatch

Performance-critical Rust dispatch paths for TPT framework backends (PyTorch, JAX).

## Overview

`tptr-dispatch` provides the hot-path Rust functions that the Python framework layers call via FFI. It routes tensor operations to `tptr-core` when hardware is available, or falls back to a pure-Rust simulation path.

## Features

- `hardware` — Link against `tptr-core` for real GPU dispatch (default: simulation fallback)

## License

Apache-2.0 — see the [repository](https://github.com/tpt-solutions/tpt-gpu) for details.
