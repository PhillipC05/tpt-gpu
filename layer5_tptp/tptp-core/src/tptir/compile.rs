//! TPTIR Kernel Compiler
//!
//! High-level interface for compiling TPTIR kernel source code.

use crate::error::{TptpError, TptpResult};
use crate::ffi::tptir_ffi;

/// Compilation target format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationTarget {
    /// TPT ISA text output
    TptIsaText = 0,
    /// LLVM IR output
    LlvmIr = 1,
    /// TPTIR binary (cached)
    TptirBinary = 2,
}

/// Compilation options
#[derive(Debug, Clone)]
pub struct CompilationOptions {
    /// Target format
    pub target: CompilationTarget,
    /// Optimization level (0-3)
    pub opt_level: u32,
    /// Enable debug info
    pub debug_info: bool,
    /// Target GPU architecture (e.g., "sm_80", "gfx1030")
    pub target_arch: Option<String>,
    /// Additional compiler flags
    pub extra_flags: Vec<String>,
}

impl Default for CompilationOptions {
    fn default() -> Self {
        CompilationOptions {
            target: CompilationTarget::TptIsaText,
            opt_level: 2,
            debug_info: false,
            target_arch: None,
            extra_flags: Vec::new(),
        }
    }
}

/// TPTIR compiler handle
pub struct TptirCompiler {
    context: tptir_ffi::TptirCompilerHandle,
    options: CompilationOptions,
}

impl TptirCompiler {
    /// Create a new TPTIR compiler with default options
    pub fn new() -> TptpResult<Self> {
        let context = tptir_ffi::TptirCompilerHandle::new()?;
        Ok(TptirCompiler {
            context,
            options: CompilationOptions::default(),
        })
    }

    /// Create a compiler with custom options
    pub fn with_options(options: CompilationOptions) -> TptpResult<Self> {
        let context = tptir_ffi::TptirCompilerHandle::new()?;
        Ok(TptirCompiler {
            context,
            options,
        })
    }

    /// Get the compiler version
    pub fn version() -> String {
        tptir_ffi::get_tptir_version()
    }

    /// Compile TPTIR source to the target format
    pub fn compile(&self, source: &str) -> TptpResult<String> {
        tptir_ffi::compile_kernel(source, self.options.target as i32)
    }

    /// Compile a specific kernel from TPTIR source
    pub fn compile_kernel(&self, source: &str, kernel_name: &str) -> TptpResult<String> {
        // In a real implementation, this would:
        // 1. Parse the source
        // 2. Find the named kernel function
        // 3. Apply optimization passes
        // 4. Generate target code
        let _ = kernel_name;
        self.compile(source)
    }

    /// Get the current compilation options
    pub fn options(&self) -> &CompilationOptions {
        &self.options
    }

    /// Set the compilation options
    pub fn set_options(&mut self, options: CompilationOptions) {
        self.options = options;
    }
}

impl Default for TptirCompiler {
    fn default() -> Self {
        Self::new().expect("failed to create TPTIR compiler")
    }
}

/// Compile a TPTIR kernel with default options (convenience function)
pub fn compile(source: &str) -> TptpResult<String> {
    let compiler = TptirCompiler::new()?;
    compiler.compile(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_version() {
        let version = TptirCompiler::version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_compiler_creation() {
        let compiler = TptirCompiler::new();
        assert!(compiler.is_ok());
    }
}