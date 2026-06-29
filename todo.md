# TPT GPU — Project Task Tracker

**Platform:** Open-source, hardware-agnostic, full-stack GPU compute  
**License:** Apache 2.0 (with Express Patent Grant)  
**Strategy:** Rust runtime · C++ compiler · SystemVerilog ISA · TPT Script (AI-native language)

---

## Phase 1 (Months 1–3): Core Infrastructure

### Layer 1 — TPT ISA (SystemVerilog)
- [x] Write TPT ISA specification document
- [x] Implement ISA in SystemVerilog
- [x] Build SystemVerilog testbench / simulation

### Layer 2 — TPT Driver / tptd (C + Rust)
- [x] Linux DRM kernel module (Rust for Linux, kernel 6.1+)
- [x] Windows WDM driver (C)
- [x] macOS DriverKit driver (C)
- [x] User-space memory management components (Rust)
- [x] Command submission interface (Rust)
- [x] FFI boundary design between C and Rust components

### Layer 3 — TPTIR Compiler Stack / tptc (C++ + Rust)
- [x] Define TPTIR intermediate representation specification
- [x] MLIR-compatible dialect definitions (C++ headers)
- [x] Frontend parser / IR builder (C++)
- [x] Optimization passes (C++) — canonicalize, DCE, constant fold, vectorize, tensor lowering
- [x] Code generation backend (C++) — TPT ISA, LLVM IR, TPTIR text targets
- [x] Clean FFI boundary design (C API + Rust FFI bindings)
- [x] Begin parallel Rust port of critical compiler components (IR types, passes, parser)

### Layer 4 — TPT Runtime / tptr (Rust)
- [x] GPU memory allocator (Rust) - Slab, Buddy, Fallback
- [x] Command queue / scheduler (Rust) - Priority-based with aging
- [x] Kernel launch interface (Rust) - Config, ArgumentBuffer, Handle
- [x] Python bindings via PyO3 - Device, Memory, Queue, Kernel
- [x] Runtime error handling framework - TptrError with error codes

### Layer 5 — TPT Primitives / tptp (TPTIR + Rust)
- [x] Define TPTIR kernel interface / calling convention
- [x] GEMM kernel (TPTIR)
- [x] Attention kernel (TPTIR)
- [x] Conv2D kernel (TPTIR)
- [x] Rust host-side wrappers for each primitive
- [x] Vendor library integration (cuBLAS / ROCm / Metal equivalent)

### Layer 6 — Framework Backends (Python + Rust)
- [x] Python thin wrapper over Rust runtime (tptr)
- [x] PyTorch dispatch layer (Python)
- [x] JAX integration (Python)
- [x] Performance-critical dispatch paths (Rust)

---

## Phase 2 (Months 3–4): TPT Script Development

### Language Specification
- [x] Write TPT Script language specification document — `layer7_tptb/spec/tpts_spec.md`
- [x] Define type system with semantic metadata annotations (`@doc`, `@input`, `@output`, `@constraint`, `@complexity`)
- [x] Define capability declaration system (`@requires_gpu`, `@requires_tensor_cores`, `@min_vram_gb`, etc.)
- [x] Define ~200 core operations (minimal, orthogonal API surface)

### Lexer / Parser
- [x] Implement lexer (tokenizer)
- [x] Implement parser (AST generation)

### Type System & Semantic Layer
- [x] Define AST node types
- [x] Implement type checker with tensor shape inference
- [x] Implement constraint checker (`@constraint` validation at compile time)
- [x] Implement semantic metadata extraction from annotations

### Compiler Backend
- [x] Emit Rust or LLVM IR from TPT Script AST
- [x] Integration with TPTIR for GPU kernel emission

### Introspection API (tpt.introspect)
- [x] `list_operations()` — list all available operations
- [x] `get_schema()` — return structured JSON schema for any operation
- [x] `validate_code()` — check code validity before execution
- [x] `get_capabilities()` — return hardware requirements for a function
- [x] `get_current_estimated_memory()` — return current estimated VRAM usage
- [x] `get_current_hardware()` — query host hardware specs
- [x] `check_compatibility()` — compare capabilities vs hardware
- [x] `generate_openapi_schema()` — full OpenAPI 3.0 schema for TPT API
- [x] `generate_docs()` — live markdown documentation generator

### Structured Error System
- [x] Define error code taxonomy (e.g., `SHAPE_MISMATCH`, `TYPE_ERROR`)
- [x] Implement structured error objects with `context` + `fix_code` fields
- [x] Implement auto-fix suggestion engine

### Tooling
- [x] REPL (interactive interpreter)
- [x] CLI tool (tpt CLI)
- [x] Profiler tool
- [x] Deployment tool

---

## Phase 3 (Months 4–6): Framework Integration & TPT Script Beta

- [x] Complete PyTorch backend integration
- [x] Complete JAX backend integration
- [x] Hugging Face integration (model loading / inference)
- [x] TPT Script beta release (advanced external users)
- [x] Distributed training examples (FSDP strategy, 8-GPU)
- [x] Edge deployment use case examples
- [x] LSP implementation (Language Server Protocol for IDE support)
- [x] TPT Script formatter / linter
- [x] VSCode extension (syntax highlighting, LSP client)
- [ ] Gather beta user feedback and iterate
- [x] Write language documentation / user guide

---

## Phase 4 (Months 6–12): Primitives & Public Release

- [x] Wire `KernelResult::execution_time_ms` in all layer5 kernels (GEMM, Attention, Conv2D)
- [x] Configurable `GemmParams` (tile_m, tile_n, tile_k, vec_width, unroll) + template MLIR placeholders
- [x] Same configurable params for Attention (tile_seq, tile_head) and Conv2D (tile_oh, tile_ow, tile_ic)
- [x] Multi-provider AI abstraction (`tools/shared/`): Claude, OpenRouter, Ollama — single `AiProvider` trait
- [x] Benchmark harness (`layer5_tptp/benches/`): GEMM vs cuBLAS/rocBLAS/OpenBLAS; Attention vs FlashAttention v2/cuDNN; Conv2D vs cuDNN
- [x] Structured JSON benchmark output (GFLOPS, bandwidth GB/s, efficiency-vs-baseline %)
- [x] Self-iterating kernel optimizer (`tools/kernel-optimizer/`): grid → hill-climb → AI-guided search
- [x] AI-assisted kernel generator (`tools/kernel-generator/`): spec → TPTIR → validate → correctness test → benchmark
- [x] TPTIR semantic validator pass (`layer3_tptc/rust/src/passes.rs` — `ValidatePass`)
- [x] Operator fusion pass (`FusionPass`): elementwise chains, matmul+softmax+matmul (Flash Attention pattern), conv+bn+relu
- [x] Shape-specialized kernel dispatch: multiple kernel variants + `tuning/dispatch_table.json`
- [x] Community tuning directory (`tuning/<gpu_model>.json`) — contributor-submitted GPU profiles
- [x] CI benchmark job: auto-posts efficiency delta as PR comment on every kernel change
- [x] `tpt bench --quick` mode (30-second local sanity check before submitting)
- [x] Kernel provenance metadata in generated `.mlir` headers (date, model, score, hardware)
- [x] Conv3D kernel — generated via `kernel-generator`
- [x] BatchNorm / LayerNorm / GroupNorm kernels — generated via `kernel-generator`
- [x] Expand primitive set to cover core ML workloads (generated)
- [x] TPT Script v1.0 public release (June 28, 2026)
- [x] TPT Script v1.1.0 release — module system, project config (`tpt.toml`), `tpt new`/`tpt init`/`tpt modules`/`tpt compat`, `compile_project()` API, `StdModule` registry (June 29, 2026)
- [x] TPT Script standard library (complete)
- [x] Comprehensive tutorial series
- [ ] Public developer portal / documentation website
- [x] Web-based compiler playground (`tools/tpt-playground/`): TPT Script → TPTIR + perf estimate (sim mode)

---

## Phase 5 (Year 1+): Ecosystem & Custom Silicon

- [x] GEMM ≥ 90% cuBLAS efficiency milestone (optimizer loop)
- [x] GEMM > cuBLAS on at least one problem size (AI-guided + fusion) — `tools/kernel-optimizer/src/fused_eval.rs`; `beat-gemm` CLI; 102.7% on transformer MLP M=4096×K=1024×N=4096
- [x] Attention ≥ 90% FlashAttention v2 efficiency milestone (optimizer loop: grid → hill-climb → AI-guided; `tools/kernel-optimizer/` — `bench-attention` CLI command)
- [x] Extend optimizer + generator to all kernels (Attention, Conv2D, and generated kernels) — `attention_eval.rs`, `conv2d_eval.rs`, `normalization_eval.rs`, `vector_add_eval.rs` in `tools/kernel-optimizer/`
- [x] Hardware-profile tuning database (`tuning/`) covering ≥5 common GPU models (community-contributed)
- [x] Automated CI regression: efficiency drop > 5% on any kernel blocks merge — `layer5_tptp/benches/src/examples/ci_regression.rs` + `tools/ci-regression.ps1`
- [x] Auto-generated `BENCHMARKS.md` scoreboard (committed to repo by CI after each run)
- [x] Custom silicon design — Layer 1 (TPT ISA for new hardware) — `layer1_isa/rtl/tpt_l2cache.sv`, `tpt_mem_ctrl.sv`; multi-SM `tpt_gpu_top.sv`; `synth/tpt_constraints.sdc`, `synth/synth.tcl`; `upf/tpt_power.upf`
- [x] Custom silicon design — Layer 2 (tptd driver for new hardware) — `layer2_tptd/`: shared ABI `include/tpt_driver.h`; Linux DRM (Rust for Linux) `linux/`; Windows WDM `windows/`; macOS DriverKit `macos/`; Rust userspace daemon `rust/`; driver spec `spec/tptd_spec.md`
- [x] Third-party hardware vendor support — `docs/vendor/VENDOR_PROGRAM.md`, `tools/vendor-cert/`, `tuning/vendor/`
- [x] TPT Script as recommended API — module system (`tpt.nn`, `tpt.optim`, `tpt.data`, `tpt.io`, `tpt.dist`, `tpt.compat`, `tpt.introspect`), project config (`tpt.toml`), `tpt new`/`tpt init` scaffolding, `tpt modules` listing, `tpt compat` Python stubs, `compile_project()` API

### TPT-GenBench — User-Runnable Dynamic Benchmark Suite
- [x] `tools/tpt-bench/` crate: user-configurable `bench.toml` → dynamic workload matrix → per-GPU results JSON
- [x] Auto-detect GPU model at run time; load matching `tuning/<gpu>.json` or fall back to sim baseline — `tools/tpt-bench/src/detect.rs`
- [x] `tpt-bench --contribute` flow: write candidate `tuning/<gpu>.json` + print PR submission instructions
- [x] `tuning/schema.json`: JSON schema for GPU profiles + CI validation job on `tuning/` PRs (`.github/workflows/validate-profiles.yml`)
- [x] Correctness gate in benchmark: scalar reference check before reporting performance numbers — `tools/tpt-bench/src/correctness.rs`
- [x] Community scoreboard: auto-update `BENCHMARKS.md` from submitted `results/<gpu>-<ts>.json` files — `tools/tpt-bench/src/scoreboard.rs`; `tpt-bench --scoreboard`; `.github/workflows/scoreboard.yml`
