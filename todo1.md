# tpt-gpu Integration Todos

These items were identified by analysing the three-repo TPT AI compute suite (tpt-gpu, tpt-spark, tpt-crucible) for cross-repo synergies. None of these are required for tpt-gpu to work standalone — they are optional improvements that strengthen the suite.

---

## 1. Publish TPTIR as a standalone crate/spec

**Why:** tpt-crucible also generates an MLIR-based IR (called TPT-IR) from its Catalyst ingestion module. If both repos share a single TPTIR dialect spec, a model compiled once to TPTIR can be routed to GPU (via tpt-gpu runtime), FPGA (via Crucible Fusion), MCU swarm (via Crucible Alloy), or analog (via Crucible Element) — without any re-compilation.

**What to do:**
- Extract the TPTIR dialect definition (ops, types, attributes) into a standalone crate (e.g. `crates/tptir-spec`) or a versioned spec document
- Define a stable text-format serialisation that tpt-crucible's Catalyst can consume
- Publish the crate to crates.io so tpt-crucible can depend on it directly
- Tag the first stable release as `tptir-spec v0.1.0`

---

## 2. Shared model registry (`~/.tpt/models/`)

**Why:** tpt-spark downloads and scans GGUF models from a local directory. tpt-crucible uses the same GGUF models as compilation inputs (for quantisation-preserving ingestion via Catalyst). Without a shared convention, users must maintain two separate model directories and potentially download models twice.

**What to do:**
- Define `~/.tpt/models/` as the canonical GGUF model directory for all TPT tools
- Define a `~/.tpt/models/models.json` manifest format: `{ "models": [{ "name": "llama-3-8b-q4", "file": "llama-3-8b-q4.gguf", "arch": "llama3", "size_gb": 4.7 }] }`
- Update tpt-gpu's HuggingFace download helper to write into `~/.tpt/models/` and update the manifest
- Document the spec in `MODELS_REGISTRY.md` at the repo root so tpt-spark and tpt-crucible can implement the same convention

---

## 3. Expose a Rust-native inference API for tpt-spark

**Why:** tpt-spark currently uses a custom `WgpuEngine` with hand-written WGSL shaders for GPU inference. tpt-gpu's Layer 4 runtime benchmarks above cuBLAS for GEMM and matches FlashAttention v2. Exposing a stable Rust API from tpt-gpu's runtime would allow tpt-spark to add a `TptGpuEngine` Cargo feature that delegates to production-quality kernels.

**What to do:**
- Design a minimal `LlmInference` Rust trait in tpt-gpu's Layer 4 runtime: `fn load(model_path: &Path) -> Result<Self>`, `fn infer(&mut self, tokens: &[u32], callback: impl Fn(u32)) -> Result<()>`, `fn cancel(&mut self)`
- Implement it for the existing GPU scheduler
- Publish as `crate tpt-gpu-runtime` (or expose via a `ffi` feature flag) so tpt-spark can add it as an optional dependency
- Write a minimal integration test that loads a GGUF model and runs inference via the trait

---

## 4. TPT Script frontend note (deferred — depends on TPTIR unification)

Once TPTIR is published as a shared spec (item 1 above) and tpt-crucible adopts it as Catalyst's output dialect, tpt-gpu's TPT Script compiler gains the ability to target FPGA, analog, and MCU swarm as compilation targets — one language for all hardware. No tpt-gpu code changes needed at that point; this is a note to revisit after item 1 is complete and tpt-crucible confirms TPTIR adoption.
