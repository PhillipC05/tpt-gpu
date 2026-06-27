//! FFI bindings to TPTIR C API
//!
//! Provides Rust wrappers around the tptir C API for kernel compilation.

pub mod tptir_ffi;

pub use tptir_ffi::{TptirCompilerHandle, TptirModule, compile_kernel};