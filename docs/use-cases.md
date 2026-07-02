# Use Cases

Runnable, end-to-end starting points, organized by what you're trying to do. Each links to the exact file or command that demonstrates it. If you haven't yet, do the root [`README.md`](../README.md#quick-start) Quick Start first.

## Write and run a GPU kernel in TPT Script

| Use case | Start here |
|---|---|
| Your first tensor program (host + GPU) | [`layer7_tptb/examples/01_hello_tensor.tpts`](../layer7_tptb/examples/01_hello_tensor.tpts) |
| Write a custom GPU kernel | [`layer7_tptb/examples/02_custom_kernel.tpts`](../layer7_tptb/examples/02_custom_kernel.tpts) |
| Multi-head attention transformer block | [`layer7_tptb/examples/03_transformer_block.tpts`](../layer7_tptb/examples/03_transformer_block.tpts) |
| Full supervised training loop | [`layer7_tptb/examples/04_training_loop.tpts`](../layer7_tptb/examples/04_training_loop.tpts) |
| FSDP across 8 GPUs | [`layer7_tptb/examples/distributed/fsdp_8gpu.tpts`](../layer7_tptb/examples/distributed/fsdp_8gpu.tpts) |
| Pipeline parallelism (4 stages) | [`layer7_tptb/examples/distributed/pipeline_parallel.tpts`](../layer7_tptb/examples/distributed/pipeline_parallel.tpts) |
| Try it live in the browser, no install | [`tools/tpt-playground/`](../tools/tpt-playground/) |

Run any `.tpts` example with `tpt check <file>` then `tpt compile <file> -o out/` — see [`layer7_tptb/examples/README.md`](../layer7_tptb/examples/README.md).

## Call TPT GPU from Python / PyTorch / JAX

| Use case | Start here |
|---|---|
| Basic Python API usage (`tptr` package) | [`layer6_framework/examples/basic_usage.py`](../layer6_framework/examples/basic_usage.py) |
| Interop with existing PyTorch code | [`layer6_framework/examples/pytorch_interop.py`](../layer6_framework/examples/pytorch_interop.py) |
| Interop with JAX | [`layer6_framework/examples/jax_interop.py`](../layer6_framework/examples/jax_interop.py) |

See also tutorials [9 (Python API)](tutorials/09_python_api.md) and [10 (PyTorch Integration)](tutorials/10_pytorch_integration.md).

## Run inference on a local LLM (GGUF)

| Use case | Start here |
|---|---|
| Download and register a GGUF model once, shared across tools | [`tools/model-registry/README.md`](../tools/model-registry/README.md) |
| Quantize/optimize a model within a quality budget | [`tools/model-optimizer/docs/developer-portal.md`](../tools/model-optimizer/docs/developer-portal.md) |
| See which model architectures are supported for inference | `layer4_tptr/tptr-core/src/arch.rs` (maps GGUF `general.architecture` → forward-pass template — LLaMA 3, Mistral, Qwen2, Phi-3, Gemma 2) |

## Benchmark and tune kernels

| Use case | Start here |
|---|---|
| Run the community benchmark suite against your GPU | [`tools/tpt-bench/README.md`](../tools/tpt-bench/README.md) — try `examples/quick_smoke.toml` first |
| Generate a new kernel from a template or AI pipeline | [`tools/kernel-generator/README.md`](../tools/kernel-generator/README.md) |
| Auto-tune a kernel's launch parameters | [`tools/kernel-optimizer/README.md`](../tools/kernel-optimizer/README.md) |
| See how TPT GEMM compares to cuBLAS | [`GEMM_VS_CUBLAS_IMPLEMENTATION.md`](../GEMM_VS_CUBLAS_IMPLEMENTATION.md) |
| Rust-level primitive benchmarks (GEMM, Attention, Conv2D/3D) | [`layer5_tptp/tptp-core/examples/`](../layer5_tptp/tptp-core/examples/) and [`layer5_tptp/benches/src/examples/`](../layer5_tptp/benches/src/examples/) |

## Certify a new hardware vendor backend

| Use case | Start here |
|---|---|
| Run Tier 1–3 certification tests against a vendor backend | [`tools/vendor-cert/README.md`](../tools/vendor-cert/README.md) |
| Read the vendor certification program requirements | [`docs/vendor/VENDOR_PROGRAM.md`](vendor/VENDOR_PROGRAM.md) |

## Going deeper

For a structured, layer-by-layer walkthrough of the whole platform (17 tutorials, ~17 hours), see [`docs/tutorials/README.md`](tutorials/README.md). For the full TPT Script language reference, see [`docs/user-guide.md`](user-guide.md).
