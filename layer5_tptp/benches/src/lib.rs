//! # TPT Primitives Benchmark Harness
//!
//! Structured benchmark output comparing TPT kernels against vendor baselines:
//! - GEMM vs cuBLAS / rocBLAS / OpenBLAS
//! - Attention vs FlashAttention v2 / cuDNN
//! - Conv2D vs cuDNN
//!
//! Output is structured JSON with GFLOPS, bandwidth GB/s, and efficiency-vs-baseline %.

pub mod kernels;
pub mod report;
pub mod harness;

pub use harness::{BenchConfig, BenchHarness, BenchResult, KernelBench};
pub use report::{BenchReport, BaselineComparison};
