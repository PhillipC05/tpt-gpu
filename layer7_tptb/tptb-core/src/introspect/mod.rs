// ---------------------------------------------------------------------------
// tpt.introspect — Introspection API for TPT Script (spec §10)
//
// Provides list_operations, get_schema, validate_code, get_capabilities,
// get_current_hardware, check_compatibility, generate_openapi_schema,
// generate_docs.  No external dependencies — JSON output is hand-built.
// ---------------------------------------------------------------------------

use std::sync::OnceLock;

use crate::ast::{Item, Program};
use crate::semantic::metadata::{extract_function_metadata, HardwareCaps};

// ---------------------------------------------------------------------------
// Schema types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ParamSchema {
    pub name: &'static str,
    pub type_str: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
pub struct ConstraintSchema {
    pub expr: &'static str,
    pub error: &'static str,
}

#[derive(Debug, Clone)]
pub struct HardwareReqs {
    pub requires_gpu: bool,
    pub requires_tensor_cores: bool,
    pub min_vram_gb: u32,
}

/// Full schema for one built-in operation.
#[derive(Debug, Clone)]
pub struct OperationSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub inputs: Vec<ParamSchema>,
    pub output_type: &'static str,
    pub output_description: &'static str,
    pub constraints: Vec<ConstraintSchema>,
    pub complexity: Option<&'static str>,
    pub differentiable: bool,
    pub gpu_optimized: bool,
    pub hardware: HardwareReqs,
    pub examples: Vec<&'static str>,
}

/// A validation error returned by [`validate_code`].
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub line: u32,
    pub col: u32,
    pub suggestion: Option<String>,
}

/// One GPU / accelerator device reported by [`get_current_hardware`].
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: u32,
    pub name: String,
    pub device_type: String,
    pub vram_gb: u32,
    pub tensor_cores: bool,
    pub compute_capability: Option<String>,
}

/// Host hardware snapshot returned by [`get_current_hardware`].
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub devices: Vec<DeviceInfo>,
    pub cpu_threads: u32,
    pub host_ram_gb: u32,
}

/// Result of [`check_compatibility`].
#[derive(Debug, Clone)]
pub struct CompatibilityResult {
    pub compatible: bool,
    pub issues: Vec<String>,
}

/// Output format for [`generate_docs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    Markdown,
    /// Python `.pyi` type stub.
    Pyi,
}

// ---------------------------------------------------------------------------
// Static operation registry
// ---------------------------------------------------------------------------

static REGISTRY: OnceLock<Vec<OperationSchema>> = OnceLock::new();

fn registry() -> &'static Vec<OperationSchema> {
    REGISTRY.get_or_init(build_registry)
}

// Compact builder helpers.
fn op(
    name: &'static str,
    description: &'static str,
    inputs: Vec<ParamSchema>,
    output_type: &'static str,
    output_description: &'static str,
    constraints: Vec<ConstraintSchema>,
    complexity: Option<&'static str>,
    differentiable: bool,
    gpu_optimized: bool,
    requires_gpu: bool,
    requires_tensor_cores: bool,
    min_vram_gb: u32,
    examples: Vec<&'static str>,
) -> OperationSchema {
    OperationSchema {
        name,
        description,
        inputs,
        output_type,
        output_description,
        constraints,
        complexity,
        differentiable,
        gpu_optimized,
        hardware: HardwareReqs { requires_gpu, requires_tensor_cores, min_vram_gb },
        examples,
    }
}

fn p(name: &'static str, type_str: &'static str, description: &'static str) -> ParamSchema {
    ParamSchema { name, type_str, description }
}

fn c(expr: &'static str, error: &'static str) -> ConstraintSchema {
    ConstraintSchema { expr, error }
}

fn simple(name: &'static str, desc: &'static str, output: &'static str, gpu: bool, diff: bool) -> OperationSchema {
    op(name, desc, vec![], output, "", vec![], None, diff, gpu, gpu, false, 0, vec![])
}

fn unary_tensor(name: &'static str, desc: &'static str, diff: bool, gpu: bool) -> OperationSchema {
    op(
        name, desc,
        vec![p("x", "Tensor[T, ...]", "Input tensor")],
        "Tensor[T, ...]", "Output tensor, same shape as input",
        vec![], None, diff, gpu, gpu, false, 0, vec![],
    )
}

fn creation(name: &'static str, desc: &'static str) -> OperationSchema {
    op(
        name, desc,
        vec![
            p("shape", "[i64, ...]", "Output shape as a list of integers"),
            p("dtype", "dtype", "Element dtype (optional, default f32)"),
        ],
        "Tensor[dtype, ...]", "New tensor filled according to the operation",
        vec![], None, false, true, false, false, 0, vec![],
    )
}

fn reduction(name: &'static str, desc: &'static str) -> OperationSchema {
    op(
        name, desc,
        vec![
            p("x", "Tensor[T, ...]", "Input tensor"),
            p("dim", "i64", "Dimension to reduce over (optional)"),
            p("keepdim", "bool", "Preserve reduced dimension as size-1 (optional)"),
        ],
        "Tensor[T, ...]", "Reduced tensor",
        vec![], None, true, true, false, false, 0, vec![],
    )
}

fn build_registry() -> Vec<OperationSchema> {
    vec![
        // ---- Tensor creation ------------------------------------------------
        creation("zeros", "Create a tensor filled with zeros"),
        creation("ones",  "Create a tensor filled with ones"),
        creation("empty", "Allocate an uninitialized tensor"),
        creation("full",  "Create a tensor filled with a scalar value"),
        creation("random","Create a tensor with uniform random values in [0, 1)"),
        creation("randn", "Create a tensor with standard-normal random values"),
        op("eye", "Create an identity matrix",
            vec![p("n", "i64", "Size of the square matrix"), p("dtype", "dtype", "Element dtype")],
            "Tensor[dtype, n, n]", "Identity matrix",
            vec![], None, false, true, false, false, 0, vec![]),
        op("arange", "Create a 1-D tensor with evenly-spaced integer values",
            vec![p("start", "i64", "Start value"), p("stop", "i64", "Exclusive end"), p("step", "i64", "Step size (optional)")],
            "Tensor[i64, n]", "Integer range tensor",
            vec![], None, false, false, false, false, 0, vec![]),
        op("linspace", "Create a 1-D tensor with n evenly-spaced values between start and stop",
            vec![p("start", "f64", "Start value"), p("stop", "f64", "End value"), p("n", "i64", "Number of points")],
            "Tensor[f64, n]", "Linearly-spaced tensor",
            vec![], None, false, false, false, false, 0, vec![]),
        op("from_list", "Construct a tensor from a nested list literal",
            vec![p("data", "[T, ...]", "Nested list of scalars")],
            "Tensor[T, ...]", "Tensor populated from the list",
            vec![], None, false, false, false, false, 0, vec![]),

        // ---- Shape-preserving unary activations -----------------------------
        unary_tensor("relu",       "Rectified linear unit: max(0, x)",                true,  true),
        unary_tensor("gelu",       "Gaussian error linear unit activation",            true,  true),
        unary_tensor("silu",       "SiLU / Swish activation: x * sigmoid(x)",         true,  true),
        unary_tensor("sigmoid",    "Element-wise sigmoid: 1 / (1 + exp(-x))",         true,  true),
        unary_tensor("tanh",       "Element-wise hyperbolic tangent",                  true,  true),
        unary_tensor("leaky_relu", "Leaky ReLU with configurable negative slope",      true,  true),
        unary_tensor("elu",        "Exponential linear unit activation",               true,  true),
        unary_tensor("sqrt",       "Element-wise square root",                         true,  true),
        unary_tensor("abs",        "Element-wise absolute value",                      true,  true),
        unary_tensor("neg",        "Element-wise negation",                            true,  true),
        unary_tensor("exp",        "Element-wise natural exponential",                 true,  true),
        unary_tensor("log",        "Element-wise natural logarithm",                   true,  true),
        unary_tensor("log2",       "Element-wise base-2 logarithm",                    true,  true),
        unary_tensor("floor",      "Element-wise floor (round toward -∞)",             false, false),
        unary_tensor("ceil",       "Element-wise ceil (round toward +∞)",              false, false),
        unary_tensor("round",      "Element-wise round to nearest integer",            false, false),
        unary_tensor("contiguous", "Return a contiguous copy of the tensor in memory", false, false),
        unary_tensor("to_host",    "Copy the tensor from device memory to host RAM",   false, false),
        op("is_nan", "Element-wise NaN test",
            vec![p("x", "Tensor[T, ...]", "Input tensor")],
            "Tensor[bool, ...]", "Boolean mask, true where x is NaN",
            vec![], None, false, false, false, false, 0, vec![]),
        op("is_inf", "Element-wise infinity test",
            vec![p("x", "Tensor[T, ...]", "Input tensor")],
            "Tensor[bool, ...]", "Boolean mask, true where x is ±Inf",
            vec![], None, false, false, false, false, 0, vec![]),

        // ---- Normalisation --------------------------------------------------
        op("softmax", "Softmax along a dimension: exp(x) / sum(exp(x))",
            vec![p("x", "Tensor[T, ...]", "Input logits"), p("dim", "i64", "Dimension to normalise over")],
            "Tensor[T, ...]", "Normalised probability distribution",
            vec![], Some("O(n)"), true, true, false, false, 0, vec![]),
        op("log_softmax", "Log-softmax: log(softmax(x)), numerically stable",
            vec![p("x", "Tensor[T, ...]", "Input logits"), p("dim", "i64", "Dimension to normalise over")],
            "Tensor[T, ...]", "Log-probability distribution",
            vec![], Some("O(n)"), true, true, false, false, 0, vec![]),
        op("normalize", "L2-normalize a tensor along a given dimension",
            vec![p("x", "Tensor[T, ...]", "Input tensor"), p("dim", "i64", "Dimension to normalise (default -1)")],
            "Tensor[T, ...]", "Unit-norm tensor",
            vec![], None, true, true, false, false, 0, vec![]),

        // ---- Type conversion ------------------------------------------------
        op("cast", "Cast tensor elements to a new dtype",
            vec![p("x", "Tensor[T, ...]", "Input tensor"), p("dtype", "dtype", "Target dtype")],
            "Tensor[dtype, ...]", "Tensor with new element type",
            vec![], None, false, false, false, false, 0, vec![]),
        op("to_device", "Move a tensor to a target compute device",
            vec![p("x", "Tensor[T, ...]", "Input tensor"), p("device", "i64", "Target device id")],
            "Tensor[T, ...]", "Tensor residing on the target device",
            vec![], None, false, false, false, false, 0, vec![]),

        // ---- Linear algebra -------------------------------------------------
        op("matmul", "General matrix multiplication: C = A × B",
            vec![
                p("a", "Tensor[T, m, k]", "Left matrix"),
                p("b", "Tensor[T, k, n]", "Right matrix"),
            ],
            "Tensor[T, m, n]", "Product matrix",
            vec![c("a.shape[1] == b.shape[0]", "Inner dimensions must match")],
            Some("O(m * n * k)"), true, true, true, false, 0,
            vec!["let c = tpt.matmul(a, b)"]),
        op("bmm", "Batched matrix multiplication: C[b] = A[b] × B[b]",
            vec![
                p("a", "Tensor[T, batch, m, k]", "Batch of left matrices"),
                p("b", "Tensor[T, batch, k, n]", "Batch of right matrices"),
            ],
            "Tensor[T, batch, m, n]", "Batch of product matrices",
            vec![
                c("a.shape[0] == b.shape[0]", "Batch sizes must match"),
                c("a.shape[3] == b.shape[2]", "Inner dimensions must match"),
            ],
            Some("O(batch * m * n * k)"), true, true, true, false, 0, vec![]),
        op("gemm", "In-place GEMM: C = alpha*A*B + beta*C (BLAS-level primitive)",
            vec![
                p("a", "Tensor[f32, m, k]", "Left matrix"),
                p("b", "Tensor[f32, k, n]", "Right matrix"),
                p("c", "Tensor[f32, m, n]", "Accumulator (modified in-place)"),
                p("alpha", "f32", "Scale for A*B (default 1.0)"),
                p("beta",  "f32", "Scale for C (default 0.0)"),
            ],
            "()", "In-place update of c",
            vec![c("a.shape[1] == b.shape[0]", "Inner dimensions must match")],
            Some("O(m * n * k)"), false, true, true, true, 0, vec![]),
        op("dot", "Dot product of two 1-D tensors",
            vec![p("a", "Tensor[T, n]", "First vector"), p("b", "Tensor[T, n]", "Second vector")],
            "T", "Scalar dot product",
            vec![c("a.shape[0] == b.shape[0]", "Vector lengths must match")],
            Some("O(n)"), true, true, false, false, 0, vec![]),
        op("outer", "Outer product of two 1-D tensors",
            vec![p("a", "Tensor[T, m]", "Row vector"), p("b", "Tensor[T, n]", "Column vector")],
            "Tensor[T, m, n]", "Outer product matrix",
            vec![], Some("O(m * n)"), true, true, false, false, 0, vec![]),
        op("det",   "Determinant of a square matrix",
            vec![p("a", "Tensor[T, n, n]", "Square matrix")],
            "T", "Scalar determinant",
            vec![c("a.shape[0] == a.shape[1]", "Matrix must be square")],
            Some("O(n^3)"), true, true, false, false, 0, vec![]),
        op("trace", "Sum of diagonal elements of a square matrix",
            vec![p("a", "Tensor[T, n, n]", "Square matrix")],
            "T", "Scalar trace",
            vec![], Some("O(n)"), true, false, false, false, 0, vec![]),
        op("inv",   "Matrix inverse",
            vec![p("a", "Tensor[T, n, n]", "Invertible square matrix")],
            "Tensor[T, n, n]", "Inverse matrix",
            vec![c("a.shape[0] == a.shape[1]", "Matrix must be square")],
            Some("O(n^3)"), true, true, false, false, 0, vec![]),
        op("svd",   "Singular value decomposition: A = U * diag(S) * Vt",
            vec![p("a", "Tensor[T, m, n]", "Input matrix")],
            "(Tensor[T, m, k], Tensor[T, k], Tensor[T, k, n])", "Tuple (U, S, V)",
            vec![], Some("O(min(m,n) * m * n)"), true, true, false, false, 0, vec![]),
        op("qr",    "QR decomposition: A = Q * R",
            vec![p("a", "Tensor[T, m, n]", "Input matrix")],
            "(Tensor[T, m, k], Tensor[T, k, n])", "Tuple (Q, R)",
            vec![], Some("O(m * n^2)"), true, true, false, false, 0, vec![]),

        // ---- Reductions -----------------------------------------------------
        reduction("sum",  "Sum all (or selected) elements"),
        reduction("mean", "Arithmetic mean over all (or selected) elements"),
        reduction("max",  "Maximum value over all (or selected) elements"),
        reduction("min",  "Minimum value over all (or selected) elements"),
        reduction("prod", "Product of all (or selected) elements"),
        reduction("norm", "L2 norm (or p-norm with p= keyword) of a tensor"),
        op("argmax", "Index of the maximum value along a dimension",
            vec![p("x", "Tensor[T, ...]", "Input tensor"), p("dim", "i64", "Dimension (optional)")],
            "Tensor[i64, ...]", "Indices of maximum values",
            vec![], None, false, false, false, false, 0, vec![]),
        op("argmin", "Index of the minimum value along a dimension",
            vec![p("x", "Tensor[T, ...]", "Input tensor"), p("dim", "i64", "Dimension (optional)")],
            "Tensor[i64, ...]", "Indices of minimum values",
            vec![], None, false, false, false, false, 0, vec![]),
        op("any", "True if any element is true (along optional dim)",
            vec![p("x", "Tensor[bool, ...]", "Boolean tensor"), p("dim", "i64", "Dimension (optional)")],
            "Tensor[bool, ...]", "Boolean reduction",
            vec![], None, false, false, false, false, 0, vec![]),
        op("all", "True if all elements are true (along optional dim)",
            vec![p("x", "Tensor[bool, ...]", "Boolean tensor"), p("dim", "i64", "Dimension (optional)")],
            "Tensor[bool, ...]", "Boolean reduction",
            vec![], None, false, false, false, false, 0, vec![]),

        // ---- Comparison / masking -------------------------------------------
        op("eq",          "Element-wise equality test",          vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[bool,...]","", vec![], None, false, false, false, false, 0, vec![]),
        op("ne",          "Element-wise inequality test",         vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[bool,...]","", vec![], None, false, false, false, false, 0, vec![]),
        op("lt",          "Element-wise less-than test",          vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[bool,...]","", vec![], None, false, false, false, false, 0, vec![]),
        op("le",          "Element-wise less-than-or-equal test", vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[bool,...]","", vec![], None, false, false, false, false, 0, vec![]),
        op("gt",          "Element-wise greater-than test",       vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[bool,...]","", vec![], None, false, false, false, false, 0, vec![]),
        op("ge",          "Element-wise greater-than-or-equal",   vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[bool,...]","", vec![], None, false, false, false, false, 0, vec![]),
        op("where",       "Select elements from x or y based on a boolean condition",
            vec![p("cond","Tensor[bool,...]","Condition mask"),p("x","Tensor[T,...]","Values where true"),p("y","Tensor[T,...]","Values where false")],
            "Tensor[T,...]","Selected values", vec![], None, true, false, false, false, 0, vec![]),
        op("masked_fill", "Fill positions where mask is true with a scalar value",
            vec![p("x","Tensor[T,...]","Input tensor"),p("mask","Tensor[bool,...]","Fill positions"),p("value","T","Fill scalar")],
            "Tensor[T,...]","Tensor with filled values", vec![], None, true, false, false, false, 0, vec![]),

        // ---- Element-wise binary --------------------------------------------
        op("add",  "Element-wise addition (broadcasts)",        vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[T,...]","", vec![], None, true, true, false, false, 0, vec![]),
        op("sub",  "Element-wise subtraction (broadcasts)",     vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[T,...]","", vec![], None, true, true, false, false, 0, vec![]),
        op("mul",  "Element-wise multiplication (broadcasts)",  vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[T,...]","", vec![], None, true, true, false, false, 0, vec![]),
        op("div",  "Element-wise division (broadcasts)",        vec![p("a","Tensor[T,...]",""),p("b","Tensor[T,...]","")], "Tensor[T,...]","", vec![], None, true, true, false, false, 0, vec![]),
        op("pow",  "Element-wise exponentiation a^b (broadcasts)", vec![p("a","Tensor[T,...]","Base"),p("b","Tensor[T,...]","Exponent")], "Tensor[T,...]","", vec![], None, true, true, false, false, 0, vec![]),
        op("clip", "Clamp tensor values to [min, max]",
            vec![p("x","Tensor[T,...]","Input"),p("min","T","Lower bound"),p("max","T","Upper bound")],
            "Tensor[T,...]","Clamped tensor", vec![], None, false, false, false, false, 0, vec![]),

        // ---- Shape manipulation ---------------------------------------------
        op("reshape", "Reshape a tensor to a new shape (must have same number of elements)",
            vec![p("x","Tensor[T,...]","Input tensor"),p("shape","[i64,...]","Target shape")],
            "Tensor[T,...]","Reshaped tensor",
            vec![c("x.numel() == product(shape)", "Total elements must be preserved")],
            None, true, false, false, false, 0, vec![]),
        op("expand", "Expand singleton dimensions to a larger size",
            vec![p("x","Tensor[T,...]","Input tensor"),p("shape","[i64,...]","Target shape (use -1 to keep)")],
            "Tensor[T,...]","Expanded tensor", vec![], None, true, false, false, false, 0, vec![]),
        op("flatten", "Flatten dimensions into a single 1-D tensor",
            vec![p("x","Tensor[T,...]","Input tensor"),p("start_dim","i64","First dim to flatten (default 0)"),p("end_dim","i64","Last dim to flatten (default -1)")],
            "Tensor[T,n]","Flattened tensor", vec![], None, true, false, false, false, 0, vec![]),
        op("squeeze",   "Remove all size-1 dimensions (or a specific one)",
            vec![p("x","Tensor[T,...]","Input tensor"),p("dim","i64","Dimension to squeeze (optional)")],
            "Tensor[T,...]","Squeezed tensor", vec![], None, true, false, false, false, 0, vec![]),
        op("unsqueeze", "Insert a size-1 dimension at a given position",
            vec![p("x","Tensor[T,...]","Input tensor"),p("dim","i64","Position to insert")],
            "Tensor[T,...]","Tensor with extra dimension", vec![], None, true, false, false, false, 0, vec![]),
        op("permute",   "Reorder dimensions according to a given permutation",
            vec![p("x","Tensor[T,...]","Input tensor"),p("dims","[i64,...]","New dimension order")],
            "Tensor[T,...]","Permuted tensor", vec![], None, true, false, false, false, 0, vec![]),
        op("transpose", "Swap two dimensions",
            vec![p("x","Tensor[T,...]","Input tensor"),p("dim0","i64","First dimension"),p("dim1","i64","Second dimension")],
            "Tensor[T,...]","Transposed tensor", vec![], None, true, false, false, false, 0, vec![]),
        op("concat", "Concatenate a list of tensors along a given dimension",
            vec![p("tensors","[Tensor[T,...],...]","List of tensors to concatenate"),p("dim","i64","Dimension to concatenate along (default 0)")],
            "Tensor[T,...]","Concatenated tensor", vec![], None, true, false, false, false, 0, vec![]),
        op("stack",  "Stack tensors along a new dimension",
            vec![p("tensors","[Tensor[T,...],...]","Tensors to stack (must have identical shape)"),p("dim","i64","New dimension position (default 0)")],
            "Tensor[T,...]","Stacked tensor",
            vec![c("all tensors have equal shape", "All input tensors must have the same shape")],
            None, true, false, false, false, 0, vec![]),
        op("split",  "Split a tensor into chunks along a dimension",
            vec![p("x","Tensor[T,...]","Input tensor"),p("size","i64","Chunk size"),p("dim","i64","Dimension to split (default 0)")],
            "[Tensor[T,...],...]","List of tensor chunks", vec![], None, true, false, false, false, 0, vec![]),
        op("chunk",  "Split a tensor into n equal-sized chunks",
            vec![p("x","Tensor[T,...]","Input tensor"),p("n","i64","Number of chunks"),p("dim","i64","Dimension to split (default 0)")],
            "[Tensor[T,...],...]","List of chunks", vec![], None, true, false, false, false, 0, vec![]),
        op("slice",  "Extract a sub-tensor using start/stop/step indices",
            vec![p("x","Tensor[T,...]","Input tensor"),p("dim","i64","Dimension"),p("start","i64","Start index"),p("stop","i64","End index (exclusive)"),p("step","i64","Step (optional)")],
            "Tensor[T,...]","Sliced tensor", vec![], None, true, false, false, false, 0, vec![]),
        op("pad",    "Pad a tensor with a constant value",
            vec![p("x","Tensor[T,...]","Input tensor"),p("padding","[i64,...]","Pad widths (pairs: before, after)"),p("value","T","Fill value (default 0)")],
            "Tensor[T,...]","Padded tensor", vec![], None, false, false, false, false, 0, vec![]),

        // ---- Convolution / pooling ------------------------------------------
        op("conv1d", "1-D convolution",
            vec![p("x","Tensor[T,N,C,L]","Input (batch, channels, length)"),p("weight","Tensor[T,F,C,K]","Filter bank"),p("bias","Tensor[T,F]","Bias (optional)")],
            "Tensor[T,N,F,L']","Convolution output", vec![], Some("O(N*F*C*K*L')"), true, true, true, false, 0, vec![]),
        op("conv2d", "2-D convolution",
            vec![p("x","Tensor[T,N,C,H,W]","Input (batch, channels, height, width)"),p("weight","Tensor[T,F,C,Kh,Kw]","Filter bank"),p("bias","Tensor[T,F]","Bias (optional)"),p("stride","[i64,i64]","Stride (default 1)"),p("padding","[i64,i64]","Padding (default 0)")],
            "Tensor[T,N,F,H',W']","Feature maps", vec![], Some("O(N*F*C*Kh*Kw*H'*W')"), true, true, true, false, 0, vec![]),
        op("conv3d", "3-D convolution",
            vec![p("x","Tensor[T,N,C,D,H,W]","Input volume"),p("weight","Tensor[T,F,C,Kd,Kh,Kw]","Filter bank")],
            "Tensor[T,N,F,D',H',W']","Output volume", vec![], None, true, true, true, false, 0, vec![]),
        op("depthwise_conv2d", "Depth-wise separable 2-D convolution",
            vec![p("x","Tensor[T,N,C,H,W]","Input"),p("weight","Tensor[T,C,1,Kh,Kw]","Per-channel filters")],
            "Tensor[T,N,C,H',W']","Depth-wise feature maps", vec![], None, true, true, true, false, 0, vec![]),
        op("conv_transpose2d", "Transposed (fractionally-strided) 2-D convolution",
            vec![p("x","Tensor[T,N,C,H,W]","Input"),p("weight","Tensor[T,C,F,Kh,Kw]","Filters")],
            "Tensor[T,N,F,H',W']","Up-sampled feature maps", vec![], None, true, true, true, false, 0, vec![]),
        op("pool2d", "2-D pooling (max or average)",
            vec![p("x","Tensor[T,N,C,H,W]","Input feature maps"),p("kernel","[i64,i64]","Pooling window size"),p("stride","[i64,i64]","Stride"),p("mode","str","\"max\" or \"avg\" (default \"max\")")],
            "Tensor[T,N,C,H',W']","Pooled feature maps", vec![], None, false, true, false, false, 0, vec![]),

        // ---- Attention ------------------------------------------------------
        op("attention", "Scaled dot-product attention: softmax(Q*Kt / sqrt(d)) * V",
            vec![
                p("q", "Tensor[T, batch, heads, seq_q, d]", "Query"),
                p("k", "Tensor[T, batch, heads, seq_k, d]", "Key"),
                p("v", "Tensor[T, batch, heads, seq_k, d_v]", "Value"),
                p("mask", "Tensor[bool, batch, heads, seq_q, seq_k]", "Attention mask (optional)"),
            ],
            "Tensor[T, batch, heads, seq_q, d_v]", "Attention output",
            vec![c("q.shape[-1] == k.shape[-1]", "Query and key depths must match")],
            Some("O(seq_q * seq_k * d)"), true, true, true, false, 0,
            vec!["let out = tpt.attention(q, k, v)"]),
        op("flash_attention", "Memory-efficient attention with O(seq) VRAM (FlashAttention algorithm)",
            vec![
                p("q", "Tensor[T, batch, heads, seq, d]", "Query"),
                p("k", "Tensor[T, batch, heads, seq, d]", "Key"),
                p("v", "Tensor[T, batch, heads, seq, d]", "Value"),
            ],
            "Tensor[T, batch, heads, seq, d]", "Attention output",
            vec![c("q.shape == k.shape", "Q, K, V must have matching shapes")],
            Some("O(seq * d)"), true, true, true, true, 8,
            vec!["let out = tpt.flash_attention(q, k, v)"]),

        // ---- Loss functions -------------------------------------------------
        op("cross_entropy", "Cross-entropy loss between logits and class indices",
            vec![p("logits","Tensor[T, batch, classes]","Unnormalised class scores"),p("targets","Tensor[i64, batch]","Ground-truth class indices")],
            "Tensor[T]","Scalar loss",
            vec![c("logits.shape[0] == targets.shape[0]", "Batch sizes must match")],
            None, true, true, false, false, 0, vec![]),
        op("mse", "Mean squared error: mean((pred - target)^2)",
            vec![p("pred","Tensor[T,...]","Predictions"),p("target","Tensor[T,...]","Targets")],
            "Tensor[T]","Scalar loss",
            vec![c("pred.shape == target.shape", "Shapes must match")],
            None, true, false, false, false, 0, vec![]),
        op("mae", "Mean absolute error: mean(|pred - target|)",
            vec![p("pred","Tensor[T,...]","Predictions"),p("target","Tensor[T,...]","Targets")],
            "Tensor[T]","Scalar loss", vec![], None, true, false, false, false, 0, vec![]),
        op("bce", "Binary cross-entropy for sigmoid outputs",
            vec![p("pred","Tensor[T,...]","Probabilities in (0,1)"),p("target","Tensor[T,...]","Binary targets in {0,1}")],
            "Tensor[T]","Scalar loss", vec![], None, true, false, false, false, 0, vec![]),
        op("kl_div", "Kullback-Leibler divergence",
            vec![p("log_pred","Tensor[T,...]","Log-probabilities (output of log_softmax)"),p("target","Tensor[T,...]","Target distribution")],
            "Tensor[T]","Scalar KL divergence", vec![], None, true, false, false, false, 0, vec![]),

        // ---- Model utilities ------------------------------------------------
        op("load_model", "Load a serialised model from disk",
            vec![p("path","str","File path to the serialised model"),p("device","i64","Target device id (optional)")],
            "Model","Loaded model object", vec![], None, false, false, false, false, 0, vec![]),
        op("save_model", "Serialise a model to disk",
            vec![p("model","Model","Model to save"),p("path","str","Output file path")],
            "()","(side effect: writes file)", vec![], None, false, false, false, false, 0, vec![]),
        simple("freeze",       "Freeze all model parameters (stop gradients)", "()", false, false),
        simple("unfreeze",     "Unfreeze all model parameters", "()", false, false),
        op("count_params", "Count total trainable parameters in a model",
            vec![p("model","Model","The model")], "i64","Parameter count", vec![], None, false, false, false, false, 0, vec![]),
        op("data_loader", "Construct a batched data loader from a dataset",
            vec![p("dataset","DataLoader","Dataset object"),p("batch_size","i64","Batch size"),p("shuffle","bool","Shuffle between epochs (default false)")],
            "DataLoader","Configured data loader", vec![], None, false, false, false, false, 0, vec![]),

        // ---- Distributed ----------------------------------------------------
        op("all_reduce", "Sum (or op) a tensor across all distributed workers",
            vec![p("x","Tensor[T,...]","Local tensor"),p("op","str","Reduction op: \"sum\", \"mean\", \"max\", \"min\" (default \"sum\")")],
            "Tensor[T,...]","Reduced tensor (same shape)", vec![], None, false, false, false, false, 0, vec![]),
        op("all_gather", "Gather tensors from all workers into a single tensor",
            vec![p("x","Tensor[T,...]","Local tensor")],
            "Tensor[T,...]","Concatenated tensor from all workers", vec![], None, false, false, false, false, 0, vec![]),
        op("broadcast", "Broadcast a tensor from one worker to all others",
            vec![p("x","Tensor[T,...]","Source tensor"),p("src","i64","Source worker rank (default 0)")],
            "Tensor[T,...]","Broadcast tensor", vec![], None, false, false, false, false, 0, vec![]),
        op("scatter", "Scatter chunks of a tensor to all workers",
            vec![p("x","Tensor[T,...]","Tensor to scatter (only meaningful on src)"),p("src","i64","Source rank (default 0)")],
            "Tensor[T,...]","Local shard", vec![], None, false, false, false, false, 0, vec![]),
        simple("barrier", "Synchronise all distributed workers at this point", "()", false, false),

        // ---- Utility --------------------------------------------------------
        op("print",     "Print a value to stdout", vec![p("x","T","Value to print")], "()", "", vec![], None, false, false, false, false, 0, vec![]),
        simple("sync",  "Wait for all pending GPU operations to complete", "()", false, false),
        op("seed",      "Set the global random seed for reproducibility", vec![p("n","i64","Seed value")], "()", "", vec![], None, false, false, false, false, 0, vec![]),
        op("shape",     "Return the shape of a tensor as a list of integers", vec![p("x","Tensor[T,...]","Input tensor")], "[i64,...]","Dimension sizes", vec![], None, false, false, false, false, 0, vec![]),
        op("dtype",     "Return the element dtype of a tensor", vec![p("x","Tensor[T,...]","Input tensor")], "dtype","Element dtype", vec![], None, false, false, false, false, 0, vec![]),
        op("device",    "Return the device id of a tensor", vec![p("x","Tensor[T,...]","Input tensor")], "i64","Device identifier", vec![], None, false, false, false, false, 0, vec![]),
        op("numel",     "Count the total number of elements in a tensor", vec![p("x","Tensor[T,...]","Input tensor")], "i64","Element count", vec![], None, false, false, false, false, 0, vec![]),
        op("benchmark", "Time a callable in milliseconds (averaged over n runs)",
            vec![p("f","fn()","Callable to benchmark"),p("n","i64","Number of repetitions (default 100)")],
            "f64","Average wall-clock time in ms", vec![], None, false, false, false, false, 0, vec![]),
        op("grad",      "Return the gradient of a tensor after backward()",
            vec![p("x","Tensor[T,...]","Tensor whose gradient to retrieve")],
            "Tensor[T,...]","Gradient tensor (same shape)", vec![], None, false, false, false, false, 0, vec![]),

        // ---- Autodiff methods -----------------------------------------------
        simple("backward",  "Compute gradients via reverse-mode autodiff (call on a scalar loss)", "()", false, false),
        simple("step",      "Apply one optimiser step (call on an optimiser object)", "()", false, false),
        simple("no_grad",   "Enter a context in which gradient tracking is disabled", "()", false, false),
        simple("forward",   "Run the forward pass of a model", "unknown", false, false),
    ]
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// List the names of all built-in `tpt.*` operations.
pub fn list_operations() -> Vec<&'static str> {
    registry().iter().map(|s| s.name).collect()
}

/// Return the schema for a named built-in operation, or `None` if unknown.
pub fn get_schema(name: &str) -> Option<&'static OperationSchema> {
    registry().iter().find(|s| s.name == name)
}

/// Lex, parse, and type-check a TPT Script source string.
///
/// Returns a (possibly empty) list of [`ValidationError`]s.  An empty list
/// means the source is syntactically and semantically valid.
pub fn validate_code(source: &str) -> Vec<ValidationError> {
    use crate::{compile_str, CompileError};
    use crate::semantic::type_check;

    match compile_str(source) {
        Err(CompileError::Lex(e)) => {
            vec![ValidationError {
                code:       "PARSE_ERROR".into(),
                message:    e.to_string(),
                line:       0,
                col:        0,
                suggestion: None,
            }]
        }
        Err(CompileError::Parse(e)) => {
            vec![ValidationError {
                code:       "PARSE_ERROR".into(),
                message:    e.to_string(),
                line:       0,
                col:        0,
                suggestion: None,
            }]
        }
        Ok(program) => {
            let checker = type_check(&program);
            checker.errors.into_iter().map(|e| ValidationError {
                code:       e.code.to_string(),
                message:    e.message,
                line:       e.span.line,
                col:        e.span.col,
                suggestion: e.suggestion,
            }).collect()
        }
    }
}

/// Extract hardware capability requirements for a named function in an already-
/// parsed `Program`.  Returns `None` if the function is not found.
pub fn get_capabilities(program: &Program, fn_name: &str) -> Option<HardwareCaps> {
    program.items.iter().find_map(|item| {
        if let Item::Function(f) = item {
            if f.name == fn_name {
                let meta = extract_function_metadata(f);
                return Some(meta.hardware);
            }
        }
        None
    })
}

/// Return a snapshot of the current host hardware.
///
/// GPU detection requires the TPT runtime (layer 4).  From the compiler layer
/// we can only observe CPU concurrency; the device list is left empty until a
/// runtime integration is provided.
pub fn get_current_hardware() -> HardwareInfo {
    let cpu_threads = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);

    HardwareInfo {
        devices: Vec::new(),
        cpu_threads,
        host_ram_gb: 0, // requires OS query, not available without external deps
    }
}

/// Check whether the given capability requirements are satisfied by a hardware
/// snapshot.  Returns a [`CompatibilityResult`] with a flag and any issues.
pub fn check_compatibility(caps: &HardwareCaps, hw: &HardwareInfo) -> CompatibilityResult {
    let mut issues = Vec::new();

    if caps.requires_gpu && hw.devices.is_empty() {
        issues.push("No GPU devices detected; function requires a GPU".into());
    }

    if caps.requires_tensor_cores {
        let has_tc = hw.devices.iter().any(|d| d.tensor_cores);
        if !has_tc {
            issues.push("Function requires tensor cores but no device with tensor cores was found".into());
        }
    }

    if caps.min_vram_gb > 0 {
        let max_vram = hw.devices.iter().map(|d| d.vram_gb).max().unwrap_or(0);
        if max_vram < caps.min_vram_gb {
            issues.push(format!(
                "Function requires at least {} GB VRAM; largest device has {} GB",
                caps.min_vram_gb, max_vram
            ));
        }
    }

    CompatibilityResult {
        compatible: issues.is_empty(),
        issues,
    }
}

/// Serialise an [`OperationSchema`] to a compact JSON string.
pub fn schema_to_json(schema: &OperationSchema) -> String {
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"name\": {},\n", json_str(schema.name)));
    out.push_str(&format!("  \"description\": {},\n", json_str(schema.description)));
    out.push_str("  \"inputs\": [\n");
    for (i, p) in schema.inputs.iter().enumerate() {
        let comma = if i + 1 < schema.inputs.len() { "," } else { "" };
        out.push_str(&format!(
            "    {{\"name\": {}, \"type\": {}, \"description\": {}}}{}\n",
            json_str(p.name), json_str(p.type_str), json_str(p.description), comma
        ));
    }
    out.push_str("  ],\n");
    out.push_str(&format!("  \"output_type\": {},\n", json_str(schema.output_type)));
    out.push_str(&format!("  \"output_description\": {},\n",
                          json_str(schema.output_description)));
    out.push_str("  \"constraints\": [\n");
    for (i, c) in schema.constraints.iter().enumerate() {
        let comma = if i + 1 < schema.constraints.len() { "," } else { "" };
        out.push_str(&format!(
            "    {{\"expr\": {}, \"error\": {}}}{}\n",
            json_str(c.expr), json_str(c.error), comma
        ));
    }
    out.push_str("  ],\n");
    out.push_str(&format!("  \"complexity\": {},\n",
        schema.complexity.map(json_str).unwrap_or_else(|| "null".into())));
    out.push_str(&format!("  \"differentiable\": {},\n", schema.differentiable));
    out.push_str(&format!("  \"gpu_optimized\": {}\n", schema.gpu_optimized));
    out.push('}');
    out
}

/// Generate a full OpenAPI 3.0.0 JSON schema for the TPT Script built-in API.
pub fn generate_openapi_schema() -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"openapi\": \"3.0.0\",\n");
    out.push_str("  \"info\": {\n");
    out.push_str("    \"title\": \"TPT Script Built-in API\",\n");
    out.push_str("    \"version\": \"1.0.0\",\n");
    out.push_str("    \"description\": \"Auto-generated schema for tpt.* operations\"\n");
    out.push_str("  },\n");
    out.push_str("  \"paths\": {\n");

    let ops = registry();
    for (i, schema) in ops.iter().enumerate() {
        let comma = if i + 1 < ops.len() { "," } else { "" };
        out.push_str(&format!("    \"/tpt/{}\": {{\n", schema.name));
        out.push_str("      \"post\": {\n");
        out.push_str(&format!("        \"summary\": {},\n", json_str(schema.description)));
        out.push_str(&format!("        \"operationId\": {},\n", json_str(schema.name)));
        // requestBody
        out.push_str("        \"requestBody\": {\n");
        out.push_str("          \"content\": { \"application/json\": { \"schema\": {\n");
        out.push_str("            \"type\": \"object\", \"properties\": {\n");
        for (pi, param) in schema.inputs.iter().enumerate() {
            let pc = if pi + 1 < schema.inputs.len() { "," } else { "" };
            out.push_str(&format!(
                "              {}: {{ \"type\": \"string\", \"description\": {} }}{}\n",
                json_str(param.name),
                json_str(if param.description.is_empty() { param.type_str } else { param.description }),
                pc
            ));
        }
        out.push_str("            }\n          }}}\n        },\n");
        // responses
        out.push_str("        \"responses\": {\n");
        out.push_str("          \"200\": {\n");
        out.push_str(&format!("            \"description\": {},\n",
            json_str(if schema.output_description.is_empty() { schema.output_type } else { schema.output_description })));
        out.push_str("            \"content\": { \"application/json\": { \"schema\": {\n");
        out.push_str(&format!("              \"type\": \"string\", \"example\": {}\n", json_str(schema.output_type)));
        out.push_str("            }}}\n");
        out.push_str("          }\n        }\n");
        out.push_str(&format!("      }}\n    }}{}\n", comma));
    }

    out.push_str("  },\n");
    out.push_str("  \"components\": {\n");
    out.push_str("    \"schemas\": {\n");
    out.push_str("      \"Tensor\": { \"type\": \"object\", \"description\": \"A multi-dimensional array with a dtype and shape\" },\n");
    out.push_str("      \"Model\": { \"type\": \"object\", \"description\": \"A trainable neural network model\" },\n");
    out.push_str("      \"DataLoader\": { \"type\": \"object\", \"description\": \"Batched data loader\" }\n");
    out.push_str("    }\n");
    out.push_str("  }\n");
    out.push_str("}\n");
    out
}

/// Generate human-readable documentation for a built-in operation.
///
/// `format` should be `DocFormat::Markdown` or `DocFormat::Pyi`.
/// Returns an empty string if the operation is not found.
pub fn generate_docs(name: &str, format: DocFormat) -> String {
    match get_schema(name) {
        None => String::new(),
        Some(schema) => match format {
            DocFormat::Markdown => render_markdown(schema),
            DocFormat::Pyi      => render_pyi(schema),
        },
    }
}

/// Generate documentation for a user-defined function using its extracted
/// [`FunctionMeta`].  Useful for tooling that has already compiled the source.
pub fn generate_docs_for_fn(
    fn_name: &str,
    meta: &crate::semantic::metadata::FunctionMeta,
    format: DocFormat,
) -> String {
    match format {
        DocFormat::Markdown => render_markdown_meta(fn_name, meta),
        DocFormat::Pyi      => render_pyi_meta(fn_name, meta),
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

fn render_markdown(s: &OperationSchema) -> String {
    let mut out = String::new();
    out.push_str(&format!("## `tpt.{}`\n\n", s.name));
    out.push_str(&format!("{}\n\n", s.description));

    if !s.inputs.is_empty() {
        out.push_str("### Parameters\n\n");
        out.push_str("| Name | Type | Description |\n|------|------|-------------|\n");
        for p in &s.inputs {
            out.push_str(&format!("| `{}` | `{}` | {} |\n", p.name, p.type_str, p.description));
        }
        out.push('\n');
    }

    out.push_str("### Returns\n\n");
    out.push_str(&format!("`{}`", s.output_type));
    if !s.output_description.is_empty() {
        out.push_str(&format!(" — {}", s.output_description));
    }
    out.push_str("\n\n");

    if !s.constraints.is_empty() {
        out.push_str("### Constraints\n\n");
        for c in &s.constraints {
            out.push_str(&format!("- `{}` — {}\n", c.expr, c.error));
        }
        out.push('\n');
    }

    if let Some(cx) = s.complexity {
        out.push_str(&format!("**Complexity:** {cx}\n\n"));
    }

    let flags: Vec<&str> = [
        s.differentiable.then_some("differentiable"),
        s.gpu_optimized.then_some("GPU-optimised"),
        s.hardware.requires_gpu.then_some("requires GPU"),
        s.hardware.requires_tensor_cores.then_some("requires tensor cores"),
    ].iter().filter_map(|x| *x).collect();
    if !flags.is_empty() {
        out.push_str(&format!("**Flags:** {}\n\n", flags.join(", ")));
    }

    if !s.examples.is_empty() {
        out.push_str("### Examples\n\n");
        for ex in &s.examples {
            out.push_str(&format!("```tpts\n{}\n```\n\n", ex));
        }
    }

    out
}

fn render_pyi(s: &OperationSchema) -> String {
    let params: Vec<String> = s.inputs.iter().map(|p| {
        let ty = tpt_type_to_python(p.type_str);
        format!("{}: {}", p.name, ty)
    }).collect();

    let ret_ty = tpt_type_to_python(s.output_type);

    let mut out = String::new();
    out.push_str(&format!("def {}({}) -> {}:\n", s.name, params.join(", "), ret_ty));
    out.push_str(&format!("    \"\"\"{}\"\"\"\n", s.description));
    out.push_str("    ...\n");
    out
}

fn render_markdown_meta(fn_name: &str, meta: &crate::semantic::metadata::FunctionMeta) -> String {
    let mut out = String::new();
    out.push_str(&format!("## `{fn_name}`\n\n"));
    if let Some(doc) = &meta.doc {
        out.push_str(&format!("{doc}\n\n"));
    }

    if !meta.inputs.is_empty() {
        out.push_str("### Parameters\n\n");
        out.push_str("| Type | Description |\n|------|-------------|\n");
        for inp in &meta.inputs {
            let desc = inp.description.as_deref().unwrap_or("");
            out.push_str(&format!("| `{}` | {} |\n", inp.type_str, desc));
        }
        out.push('\n');
    }

    if let Some(o) = &meta.output {
        out.push_str("### Returns\n\n");
        out.push_str(&format!("`{}`", o.type_str));
        if let Some(d) = &o.description {
            out.push_str(&format!(" — {d}"));
        }
        out.push_str("\n\n");
    }

    if !meta.constraints.is_empty() {
        out.push_str("### Constraints\n\n");
        for c in &meta.constraints {
            let err = c.error_msg.as_deref().unwrap_or("");
            out.push_str(&format!("- `{}` — {err}\n", c.expr_str));
        }
        out.push('\n');
    }

    if let Some(cx) = &meta.complexity {
        out.push_str(&format!("**Complexity:** {cx}\n\n"));
    }

    let hw = &meta.hardware;
    let flags: Vec<String> = [
        meta.differentiable.unwrap_or(false).then_some("differentiable"),
        hw.gpu_optimized.then_some("GPU-optimised"),
        hw.requires_gpu.then_some("requires GPU"),
        hw.requires_tensor_cores.then_some("requires tensor cores"),
    ].iter().filter_map(|x| x.as_ref().map(|s| s.to_string())).collect();
    if !flags.is_empty() {
        out.push_str(&format!("**Flags:** {}\n\n", flags.join(", ")));
    }

    if !meta.examples.is_empty() {
        out.push_str("### Examples\n\n");
        for ex in &meta.examples {
            out.push_str(&format!("```tpts\n{ex}\n```\n\n"));
        }
    }

    out
}

fn render_pyi_meta(fn_name: &str, meta: &crate::semantic::metadata::FunctionMeta) -> String {
    let params: Vec<String> = meta.inputs.iter().enumerate().map(|(i, inp)| {
        format!("arg{}: {}", i, tpt_type_to_python(&inp.type_str))
    }).collect();
    let ret_ty = meta.output.as_ref()
        .map(|o| tpt_type_to_python(&o.type_str))
        .unwrap_or_else(|| "None".into());
    let doc = meta.doc.as_deref().unwrap_or("");

    format!("def {}({}) -> {}:\n    \"\"\"{doc}\"\"\"\n    ...\n",
        fn_name, params.join(", "), ret_ty)
}

fn tpt_type_to_python(tpt_ty: &str) -> String {
    // Best-effort mapping of TPT type syntax to Python stub syntax.
    let t = tpt_ty.trim();
    if t.starts_with("Tensor[") {
        "\"Tensor\"".into()
    } else if t.starts_with("(") || t.starts_with("[") {
        "tuple".into()
    } else {
        match t {
            "f32" | "f64" => "float".into(),
            "i32" | "i64" | "u32" | "u64" | "i8" | "i16" | "u8" | "u16" | "index" => "int".into(),
            "bool" => "bool".into(),
            "()" => "None".into(),
            "str" => "str".into(),
            "Model" => "\"Model\"".into(),
            "DataLoader" => "\"DataLoader\"".into(),
            other => format!("\"{}\"", other),
        }
    }
}

// ---------------------------------------------------------------------------
// JSON string helper
// ---------------------------------------------------------------------------

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c    => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// OperationSchema → JSON
// ---------------------------------------------------------------------------

impl OperationSchema {
    /// Serialise this schema to JSON following the format specified in §10.4.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"name\": {},\n", json_str(self.name)));
        out.push_str(&format!("  \"description\": {},\n", json_str(self.description)));

        // inputs
        out.push_str("  \"inputs\": [\n");
        for (i, inp) in self.inputs.iter().enumerate() {
            let comma = if i + 1 < self.inputs.len() { "," } else { "" };
            out.push_str(&format!(
                "    {{ \"name\": {}, \"type\": {}, \"description\": {} }}{}\n",
                json_str(inp.name), json_str(inp.type_str), json_str(inp.description), comma
            ));
        }
        out.push_str("  ],\n");

        // output
        out.push_str(&format!(
            "  \"output\": {{ \"type\": {}, \"description\": {} }},\n",
            json_str(self.output_type), json_str(self.output_description)
        ));

        // constraints
        out.push_str("  \"constraints\": [\n");
        for (i, c) in self.constraints.iter().enumerate() {
            let comma = if i + 1 < self.constraints.len() { "," } else { "" };
            out.push_str(&format!(
                "    {{ \"expr\": {}, \"error\": {} }}{}\n",
                json_str(c.expr), json_str(c.error), comma
            ));
        }
        out.push_str("  ],\n");

        // scalar fields
        match self.complexity {
            Some(cx) => out.push_str(&format!("  \"complexity\": {},\n", json_str(cx))),
            None     => out.push_str("  \"complexity\": null,\n"),
        }
        out.push_str(&format!("  \"differentiable\": {},\n", self.differentiable));
        out.push_str(&format!("  \"gpu_optimized\": {},\n", self.gpu_optimized));

        // hardware
        out.push_str("  \"hardware\": {\n");
        out.push_str(&format!("    \"requires_gpu\": {},\n", self.hardware.requires_gpu));
        out.push_str(&format!("    \"requires_tensor_cores\": {},\n", self.hardware.requires_tensor_cores));
        out.push_str(&format!("    \"min_vram_gb\": {}\n", self.hardware.min_vram_gb));
        out.push_str("  },\n");

        // examples
        out.push_str("  \"examples\": [\n");
        for (i, ex) in self.examples.iter().enumerate() {
            let comma = if i + 1 < self.examples.len() { "," } else { "" };
            out.push_str(&format!("    {}{}\n", json_str(ex), comma));
        }
        out.push_str("  ]\n");

        out.push_str("}\n");
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_operations_non_empty() {
        let ops = list_operations();
        assert!(!ops.is_empty());
        assert!(ops.contains(&"matmul"));
        assert!(ops.contains(&"relu"));
        assert!(ops.contains(&"attention"));
        assert!(ops.contains(&"cross_entropy"));
    }

    #[test]
    fn test_list_operations_all_unique() {
        let mut ops = list_operations();
        let len = ops.len();
        ops.sort_unstable();
        ops.dedup();
        assert_eq!(ops.len(), len, "duplicate operation names in registry");
    }

    #[test]
    fn test_get_schema_matmul() {
        let s = get_schema("matmul").expect("matmul should be in registry");
        assert_eq!(s.name, "matmul");
        assert_eq!(s.inputs.len(), 2);
        assert_eq!(s.inputs[0].name, "a");
        assert_eq!(s.inputs[1].name, "b");
        assert!(s.differentiable);
        assert!(s.hardware.requires_gpu);
        assert!(!s.constraints.is_empty());
    }

    #[test]
    fn test_get_schema_unknown_returns_none() {
        assert!(get_schema("does_not_exist").is_none());
    }

    #[test]
    fn test_validate_code_valid() {
        let errs = validate_code("fn f(x: Tensor[f32, m, n]) -> Tensor[f32, m, n] { return tpt.relu(x) }");
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }

    #[test]
    fn test_validate_code_syntax_error() {
        let errs = validate_code("fn f( { }");
        assert!(!errs.is_empty());
        assert_eq!(errs[0].code, "PARSE_ERROR");
    }

    #[test]
    fn test_validate_code_type_error() {
        let errs = validate_code("fn f() { let x = unknown_var }");
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|e| e.code == "UNDEFINED_VARIABLE"));
    }

    #[test]
    fn test_get_capabilities() {
        let src = r#"
@requires_gpu(true)
@requires_tensor_cores(true)
@min_vram_gb(16)
fn big_model(x: Tensor[f32, batch, seq]) -> Tensor[f32, batch, seq] {
    return tpt.relu(x)
}
"#;
        let prog = crate::compile_str(src).unwrap();
        let caps = get_capabilities(&prog, "big_model").expect("function should be found");
        assert!(caps.requires_gpu);
        assert!(caps.requires_tensor_cores);
        assert_eq!(caps.min_vram_gb, 16);
    }

    #[test]
    fn test_get_capabilities_unknown_fn() {
        let prog = crate::compile_str("fn f() {}").unwrap();
        assert!(get_capabilities(&prog, "does_not_exist").is_none());
    }

    #[test]
    fn test_get_current_hardware() {
        let hw = get_current_hardware();
        assert!(hw.cpu_threads >= 1);
    }

    #[test]
    fn test_check_compatibility_no_gpu() {
        use crate::semantic::metadata::HardwareCaps;
        let caps = HardwareCaps { requires_gpu: true, ..Default::default() };
        let hw = HardwareInfo { devices: vec![], cpu_threads: 4, host_ram_gb: 16 };
        let result = check_compatibility(&caps, &hw);
        assert!(!result.compatible);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_check_compatibility_ok() {
        use crate::semantic::metadata::HardwareCaps;
        let caps = HardwareCaps { requires_gpu: false, min_vram_gb: 0, ..Default::default() };
        let hw = HardwareInfo { devices: vec![], cpu_threads: 4, host_ram_gb: 16 };
        let result = check_compatibility(&caps, &hw);
        assert!(result.compatible);
    }

    #[test]
    fn test_check_compatibility_vram_insufficient() {
        use crate::semantic::metadata::HardwareCaps;
        let caps = HardwareCaps { requires_gpu: true, min_vram_gb: 24, ..Default::default() };
        let hw = HardwareInfo {
            devices: vec![DeviceInfo {
                id: 0, name: "TestGPU".into(), device_type: "gpu".into(),
                vram_gb: 8, tensor_cores: false, compute_capability: None,
            }],
            cpu_threads: 8, host_ram_gb: 64,
        };
        let result = check_compatibility(&caps, &hw);
        assert!(!result.compatible);
        assert!(result.issues.iter().any(|i| i.contains("VRAM")));
    }

    #[test]
    fn test_generate_openapi_schema_is_json() {
        let json = generate_openapi_schema();
        assert!(json.contains("\"openapi\": \"3.0.0\""));
        assert!(json.contains("matmul"));
        assert!(json.contains("attention"));
        // Balanced braces (rough check)
        let opens  = json.chars().filter(|&c| c == '{').count();
        let closes = json.chars().filter(|&c| c == '}').count();
        assert_eq!(opens, closes, "unbalanced braces in OpenAPI output");
    }

    #[test]
    fn test_schema_to_json_matmul() {
        let s = get_schema("matmul").unwrap();
        let json = s.to_json();
        assert!(json.contains("\"name\": \"matmul\""));
        assert!(json.contains("\"differentiable\": true"));
        assert!(json.contains("\"requires_gpu\": true"));
        assert!(json.contains("\"complexity\": \"O(m * n * k)\""));
    }

    #[test]
    fn test_generate_docs_markdown() {
        let md = generate_docs("attention", DocFormat::Markdown);
        assert!(md.contains("## `tpt.attention`"));
        assert!(md.contains("### Parameters"));
        assert!(md.contains("### Returns"));
        assert!(md.contains("complexity") || md.contains("Complexity"));
    }

    #[test]
    fn test_generate_docs_pyi() {
        let pyi = generate_docs("matmul", DocFormat::Pyi);
        assert!(pyi.contains("def matmul("));
        assert!(pyi.contains("-> "));
        assert!(pyi.contains("..."));
    }

    #[test]
    fn test_generate_docs_unknown_returns_empty() {
        assert_eq!(generate_docs("not_an_op", DocFormat::Markdown), "");
    }
}
