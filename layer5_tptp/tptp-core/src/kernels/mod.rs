//! Kernel Host Wrappers
//!
//! High-level Rust wrappers for GPU compute kernels (GEMM, Attention, Conv2D).
//! Each kernel validates inputs, dispatches to vendor library or TPTIR fallback,
//! and manages output buffer allocation.

pub mod gemm;
pub mod attention;
pub mod conv2d;

pub use gemm::GemmKernel;
pub use attention::AttentionKernel;
pub use conv2d::Conv2DKernel;