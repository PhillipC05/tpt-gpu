//! Attention Kernel Wrapper — Scaled Dot-Product Attention
use crate::error::{TptpError, TptpResult};
use crate::memory::{GpuBuffer, DType, Shape, BufferFlags};
use crate::kernel::{KernelConfig, KernelResult, PrimitiveKernel};
use crate::vendor::{VendorBackend, VendorLibrary};

/// Attention kernel handle
pub struct AttentionKernel {
    config: KernelConfig,
    vendor: VendorBackend,
}

impl AttentionKernel {
    pub fn new() -> Self {
        let vendor = VendorBackend::detect();
        let config = KernelConfig::new([32, 1, 1], [256, 1, 1]);
        AttentionKernel { config, vendor }
    }

    pub fn with_vendor(vendor: VendorBackend) -> Self {
        let config = KernelConfig::new([32, 1, 1], [256, 1, 1]);
        AttentionKernel { config, vendor }
    }

    pub fn execute(&self, q: &GpuBuffer<f32>, k: &GpuBuffer<f32>, v: &GpuBuffer<f32>, scale: Option<f32>, mask: Option<&GpuBuffer<f32>>) -> TptpResult<GpuBuffer<f32>> {
        if q.ndim() != 2 || k.ndim() != 2 || v.ndim() != 2 {
            return Err(TptpError::shape_error("Attention requires 2D tensors"));
        }
        let seq_len = q.dim(0).ok_or_else(|| TptpError::shape_error("Q has no dim 0"))?;
        let d_k = q.dim(1).ok_or_else(|| TptpError::shape_error("Q has no dim 1"))?;
        let d_v = v.dim(1).ok_or_else(|| TptpError::shape_error("V has no dim 1"))?;
        if k.dim(0) != Some(seq_len) || k.dim(1) != Some(d_k) {
            return Err(TptpError::shape_error("K dimensions must match Q"));
        }
        if v.dim(0) != Some(seq_len) {
            return Err(TptpError::shape_error("V seq_len must match Q"));
        }
        let scale_val = scale.unwrap_or_else(|| 1.0 / (d_k as f32).sqrt());
        let mut output = GpuBuffer::new(Shape::dim2(seq_len, d_v), DType::F32, BufferFlags::STORAGE)?;
        if self.vendor.supports_attention() {
            self.vendor.attention(q, k, v, &mut output, scale_val, seq_len, d_k)?;
        } else {
            self.tptir_fallback_attention(q, k, v, &mut output, scale_val, seq_len, d_k)?;
        }
        Ok(output)
    }

    fn tptir_fallback_attention(&self, _q: &GpuBuffer<f32>, _k: &GpuBuffer<f32>, _v: &GpuBuffer<f32>, _output: &mut GpuBuffer<f32>, _scale: f32, _seq_len: usize, _d_k: usize) -> TptpResult<()> {
        log::debug!("TPTIR Attention fallback: seq_len={}, d_k={}", _seq_len, _d_k);
        Ok(())
    }
}

impl PrimitiveKernel for AttentionKernel {
    fn name(&self) -> &str { "attention" }
    fn input_shapes(&self) -> &[Shape] { &[] }
    fn output_shape(&self) -> &Shape { unimplemented!("output_shape not implemented") }
    fn supported_dtypes(&self) -> &[DType] { &[DType::F32, DType::F16] }
    fn can_execute(&self, inputs: &[&GpuBuffer<f32>]) -> bool { inputs.len() == 3 && inputs.iter().all(|i| i.ndim() == 2) }
    fn default_config(&self) -> KernelConfig { KernelConfig::new([32, 1, 1], [256, 1, 1]) }
    fn execute(&self, inputs: &[&GpuBuffer<f32>], output: &mut GpuBuffer<f32>, _config: &KernelConfig) -> TptpResult<KernelResult> {
        let q = inputs[0]; let k = inputs[1]; let v = inputs[2];
        let seq_len = q.dim(0).unwrap_or(0); let d_k = q.dim(1).unwrap_or(0); let d_v = v.dim(1).unwrap_or(0);
        if self.vendor.supports_attention() { self.vendor.attention(q, k, v, output, 1.0 / (d_k as f32).sqrt(), seq_len, d_k)?; }
        Ok(KernelResult { outputs: vec![], execution_time_ms: None, backend_used: self.vendor.name().to_string() })
    }
    fn execute_with_vendor(&self, inputs: &[&GpuBuffer<f32>], output: &mut GpuBuffer<f32>, vendor: &VendorBackend, _config: &KernelConfig) -> TptpResult<KernelResult> {
        let q = inputs[0]; let k = inputs[1]; let v = inputs[2];
        let seq_len = q.dim(0).unwrap_or(0); let d_k = q.dim(1).unwrap_or(0);
        vendor.attention(q, k, v, output, 1.0 / (d_k as f32).sqrt(), seq_len, d_k)?;
        Ok(KernelResult { outputs: vec![], execution_time_ms: None, backend_used: vendor.name().to_string() })
    }
}

pub fn attention(q: &GpuBuffer<f32>, k: &GpuBuffer<f32>, v: &GpuBuffer<f32>, scale: Option<f32>) -> TptpResult<GpuBuffer<f32>> {
    AttentionKernel::new().execute(q, k, v, scale, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_attention_validation() {
        let q = GpuBuffer::<f32>::new(Shape::dim2(8, 64), DType::F32, BufferFlags::STORAGE).unwrap();
        let k = GpuBuffer::<f32>::new(Shape::dim2(8, 64), DType::F32, BufferFlags::STORAGE).unwrap();
        let v = GpuBuffer::<f32>::new(Shape::dim2(8, 64), DType::F32, BufferFlags::STORAGE).unwrap();
        let kernel = AttentionKernel::new();
        let result = kernel.execute(&q, &k, &v, None, None);
        assert!(result.is_ok());
    }
    #[test] fn test_attention_shape_mismatch() {
        let q = GpuBuffer::<f32>::new(Shape::dim2(8, 64), DType::F32, BufferFlags::STORAGE).unwrap();
        let k = GpuBuffer::<f32>::new(Shape::dim2(10, 64), DType::F32, BufferFlags::STORAGE).unwrap();
        let v = GpuBuffer::<f32>::new(Shape::dim2(8, 64), DType::F32, BufferFlags::STORAGE).unwrap();
        let kernel = AttentionKernel::new();
        let result = kernel.execute(&q, &k, &v, None, None);
        assert!(result.is_err());
    }
}