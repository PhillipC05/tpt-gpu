"""
PyTorch operation dispatch to TPT runtime.

Maps PyTorch ATen operations to TPT kernel launches.
"""
from __future__ import annotations
from typing import Any, Dict, Optional, Tuple
from tptr._ffi import TptrError


# Mapping from PyTorch op names to TPT kernel names
_OP_MAP: Dict[str, str] = {
    "aten.add.Tensor": "add",
    "aten.add.Scalar": "add",
    "aten.mul.Tensor": "mul",
    "aten.mul.Scalar": "mul",
    "aten.sub.Tensor": "sub",
    "aten.sub.Scalar": "sub",
    "aten.div.Tensor": "div",
    "aten.div.Scalar": "div",
    "aten.neg.default": "neg",
    "aten.relu.default": "relu",
    "aten.gelu.default": "gelu",
    "aten.silu.default": "silu",
    "aten.softmax.int": "softmax",
    "aten.sum.dim_IntList": "sum",
    "aten.mean.dim": "mean",
    "aten.mm.default": "matmul",
    "aten.bmm.default": "matmul",
    "aten.layer_norm.default": "layer_norm",
}

# Ops that support in-place modification
_INPLACE_OPS = {
    "aten.add_.Tensor",
    "aten.add_.Scalar",
    "aten.mul_.Tensor",
    "aten.mul_.Scalar",
    "aten.relu_.default",
}


def dispatch_op(op: str, args: tuple, kwargs: dict) -> Any:
    """
    Dispatch a PyTorch operation to TPT runtime.

    Args:
        op: The PyTorch ATen op name (e.g., "aten.add.Tensor")
        args: Positional arguments (tensors, scalars)
        kwargs: Keyword arguments

    Returns:
        Result of the operation (simulated)
    """
    tpt_op = _OP_MAP.get(op)
    if tpt_op is None:
        raise NotImplementedError(f"TPT does not support PyTorch op: {op}")

    # In a real implementation, this would:
    # 1. Extract tensor data from PyTorch tensors
    # 2. Allocate TPT memory if needed
    # 3. Launch the TPT kernel
    # 4. Return a new PyTorch tensor backed by TPT memory

    # For now, return a placeholder result
    return _execute_tpt_op(tpt_op, args, kwargs)


def _execute_tpt_op(tpt_op: str, args: tuple, kwargs: dict) -> Any:
    """Execute a TPT operation (simulated)."""
    # This is where the actual TPT kernel launch would happen
    # For the simulation, we return None as a placeholder
    return None


def get_supported_ops() -> list:
    """Get list of PyTorch ops supported by TPT."""
    return sorted(_OP_MAP.keys())


def is_supported(op: str) -> bool:
    """Check if a PyTorch op is supported by TPT."""
    return op in _OP_MAP


def get_tpt_op_name(op: str) -> Optional[str]:
    """Get the TPT kernel name for a PyTorch op."""
    return _OP_MAP.get(op)

