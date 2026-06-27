//! # TPT Primitives (tptp-core)
//!
//! GPU compute primitives for the TPT GPU platform.
//! Provides high-level Rust wrappers for GEMM, Attention, Conv2D kernels
//! with TPTIR compilation and vendor library dispatch.

pub mod error;
pub mod kernel;
pub mod memory;
pub mod ffi;
pub mod kernels;
pub mod vendor;
pub mod tptir;

pub use error::{TptpError, TptpResult};
pub use kernel::{PrimitiveKernel, KernelConfig, KernelDispatch};
pub use memory::{GpuBuffer, BufferFlags, DType};
pub use kernels::{GemmKernel, AttentionKernel, Conv2DKernel};
pub use vendor::{VendorBackend, VendorLibrary};
pub use tptir::{TptirCompiler, CompilationOptions, CompilationTarget};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{TptpError, TptpResult, GpuBuffer, DType, BufferFlags};
    pub use crate::kernel::{PrimitiveKernel, KernelConfig};
    pub use crate::kernels::{GemmKernel, AttentionKernel, Conv2DKernel};
    pub use crate::vendor::VendorBackend;
}

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name
pub const NAME: &str = env!("CARGO_PKG_NAME");