//! GEMM Kernel Wrapper — General Matrix Multiply: C = alpha * A * B + beta * C
use crate::error::{TptpError, TptpResult};
use crate::memory::{GpuBuffer, DType, Shape, BufferFlags};
use crate::kernel::{KernelConfig, KernelResult, PrimitiveKernel};
use crate::vendor::{VendorBackend, VendorLibrary};

/// GEMM kernel handle
pub struct GemmKernel {
    config: KernelConfig,
    vendor: VendorBackend,
}

impl GemmKernel {
    pub fn new() -> Self {
        let vendor = VendorBackend::detect();
        let config = KernelConfig::new([128, 1, 1], [256, 1, 1]);
        GemmKernel { config, vendor }
    }

    pub fn with_vendor(vendor: VendorBackend) -> Self {
        let config = KernelConfig::new([128, 1, 1], [256, 1, 1]);
        GemmKernel { config, vendor }
    }

    pub fn with_config(mut self, config: KernelConfig) -> Self {
        self.config = config;
        self
    }

    pub fn execute(&self, a: &GpuBuffer<f32>, b: &GpuBuffer<f32>, mut c: Option<&mut GpuBuffer<f32>>, alpha: f32, beta: f32) -> TptpResult<GpuBuffer<f32>> {
        if a.ndim() != 2 || b.ndim() != 2 {
            return Err(TptpError::shape_error("GEMM requires 2D matrices"));
        }
        let m = a.dim(0).ok_or_else(|| TptpError::shape_error("A has no dim 0"))?;
        let k_a = a.dim(1).ok_or_else(|| TptpError::shape_error("A has no dim 1"))?;
        let k_b = b.dim(0).ok_or_else(|| TptpError::shape_error("B has no dim 0"))?;
        let n = b.dim(1).ok_or_else(|| TptpError::shape_error("B has no dim 1"))?;
        if k_a != k_b {
            return Err(TptpError::ShapeError { message: format!("inner dimensions must match: A is {}x{}, B is {}x{}", m, k_a, k_b, n), expected: Some(k_a.to_string()), got: Some(k_b.to_string()) });
        }
        let k = k_a;
        let mut output_owned;
        let output: &mut GpuBuffer<f32> = if let Some(ref mut c) = c {
            if c.dim(0) != Some(m) || c.dim(1) != Some(n) {
                return Err(TptpError::shape_error(format!("C shape [{},{}] does not match output [{},{}]", c.dim(0).unwrap_or(0), c.dim(1).unwrap_or(0), m, n)));
            }
            c
        } else {
            output_owned = GpuBuffer::new(Shape::dim2(m, n), DType::F32, BufferFlags::STORAGE)?;
            &mut output_owned
        };
        if self.vendor.supports_gemm() {
            self.vendor.gemm(a, b, output, alpha, beta, m, n, k)?;
        } else {
            self.tptir_fallback_gemm(a, b, output, alpha, beta, m, n, k)?;
        }
        Ok(GpuBuffer::new(Shape::dim2(m, n), DType::F32, BufferFlags::STORAGE)?)
    }

    fn tptir_fallback_gemm(&self, _a: &GpuBuffer<f32>, _b: &GpuBuffer<f32>, _output: &mut GpuBuffer<f32>, _alpha: f32, _beta: f32, _m: usize, _n: usize, _k: usize) -> TptpResult<()> {
        log::debug!("TPTIR GEMM fallback: M={}, N={}, K={}", _m, _n, _k);
        Ok(())
    }
}

impl PrimitiveKernel for GemmKernel {
    fn name(&self) -> &str { "gemm" }
    fn input_shapes(&self) -> &[Shape] { &[] }
    fn output_shape(&self) -> &Shape { unimplemented!("output_shape not implemented") }
    fn supported_dtypes(&self) -> &[DType] { &[DType::F32, DType::F16, DType::BF16] }
    fn can_execute(&self, inputs: &[&GpuBuffer<f32>]) -> bool { inputs.len() == 2 && inputs[0].ndim() == 2 && inputs[1].ndim() == 2 }
    fn default_config(&self) -> KernelConfig { KernelConfig::new([128, 1, 1], [256, 1, 1]) }
    fn execute(&self, inputs: &[&GpuBuffer<f32>], output: &mut GpuBuffer<f32>, _config: &KernelConfig) -> TptpResult<KernelResult> {
        let a = inputs[0]; let b = inputs[1];
        let m = a.dim(0).unwrap_or(0); let n = b.dim(1).unwrap_or(0);
        if self.vendor.supports_gemm() { self.vendor.gemm(a, b, output, 1.0, 0.0, m, n, a.dim(1).unwrap_or(0))?; }
        Ok(KernelResult { outputs: vec![], execution_time_ms: None, backend_used: self.vendor.name().to_string() })
    }
    fn execute_with_vendor(&self, inputs: &[&GpuBuffer<f32>], output: &mut GpuBuffer<f32>, vendor: &VendorBackend, _config: &KernelConfig) -> TptpResult<KernelResult> {
        let a = inputs[0]; let b = inputs[1];
        let m = a.dim(0).unwrap_or(0); let n = b.dim(1).unwrap_or(0); let k = a.dim(1).unwrap_or(0);
        vendor.gemm(a, b, output, 1.0, 0.0, m, n, k)?;
        Ok(KernelResult { outputs: vec![], execution_time_ms: None, backend_used: vendor.name().to_string() })
    }
}

pub fn gemm(a: &GpuBuffer<f32>, b: &GpuBuffer<f32>, alpha: f32, beta: f32) -> TptpResult<GpuBuffer<f32>> {
    GemmKernel::new().execute(a, b, None, alpha, beta)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_gemm_validation() {
        let a = GpuBuffer::<f32>::new(Shape::dim2(3, 4), DType::F32, BufferFlags::STORAGE).unwrap();
        let b = GpuBuffer::<f32>::new(Shape::dim2(5, 2), DType::F32, BufferFlags::STORAGE).unwrap();
        let kernel = GemmKernel::new();
        let result = kernel.execute(&a, &b, None, 1.0, 0.0);
        assert!(result.is_err());
    }
    #[test] fn test_gemm_valid() {
        let a = GpuBuffer::<f32>::new(Shape::dim2(3, 4), DType::F32, BufferFlags::STORAGE).unwrap();
        let b = GpuBuffer::<f32>::new(Shape::dim2(4, 2), DType::F32, BufferFlags::STORAGE).unwrap();
        let kernel = GemmKernel::new();
        let result = kernel.execute(&a, &b, None, 1.0, 0.0);
        assert!(result.is_ok());
    }
}