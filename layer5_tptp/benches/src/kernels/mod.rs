//! Kernel benchmark implementations
//!
//! Each kernel implements `KernelBench` to provide problem sizes,
//! theoretical GFLOPS calculations, and timed execution.

pub mod gemm;
pub mod attention;
pub mod conv2d;

pub use gemm::GemmBench;
pub use attention::AttentionBench;
pub use conv2d::Conv2DBench;
