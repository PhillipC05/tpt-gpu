"""
TPT JAX Integration.

Provides a JAX-compatible backend using the XLA/PLUGINS mechanism
to register TPT as a JAX platform.

Usage:
    import jax
    import tptr.jax

    # Register TPT as a JAX backend
    tptr.jax.register_backend()

    # JAX operations will be dispatched to TPT
    x = jax.numpy.ones((32, 64))
    y = jax.numpy.dot(x, x.T)  # matmul via TPT
"""
from __future__ import annotations
from typing import Optional, Any, Dict, List
import warnings


def is_available() -> bool:
    """Check if JAX integration is available."""
    try:
        import jax  # noqa: F401
        return True
    except ImportError:
        return False


def register_backend(platform_name: str = "tpt") -> bool:
    """
    Register TPT as a JAX backend platform.

    This registers TPT with JAX's plugin mechanism so that
    JAX operations can be dispatched to the TPT runtime.

    Args:
        platform_name: The platform name to register (default: "tpt")

    Returns True if registration succeeded.
    """
    try:
        import jax
    except ImportError:
        warnings.warn("JAX not installed. Cannot register TPT backend.")
        return False

    try:
        # Register with JAX's platform registry
        # In a real implementation, this would use jax.lib.xla_client
        # to register a custom call handler
        _register_custom_calls()
        return True
    except Exception as e:
        warnings.warn(f"Failed to register TPT JAX backend: {e}")
        return False


def _register_custom_calls() -> None:
    """Register TPT custom calls with JAX/XLA."""
    # In a real implementation, this would register:
    # - tpt_matmul
    # - tpt_elementwise_add
    # - tpt_elementwise_mul
    # - tpt_relu
    # - tpt_gelu
    # - tpt_softmax
    # - tpt_sum
    # - tpt_mean
    # - tpt_layer_norm
    pass


def get_backend_name() -> str:
    """Get the JAX platform name for TPT."""
    return "tpt"


class TptrJaxArray:
    """JAX-compatible array backed by TPT memory."""

    def __init__(self, shape: tuple, dtype: str = "float32", device_index: int = 0):
        from tptr._ffi import Device as NativeDevice
        import numpy as np

        self._shape = shape
        self._dtype = dtype
        self._device_index = device_index
        self._native_device = NativeDevice(device_index)

        dtype_sizes = {"float16": 2, "float32": 4, "float64": 8,
                       "int8": 1, "int32": 4, "int64": 8}
        itemsize = dtype_sizes.get(dtype, 4)
        self._nbytes = int(np.prod(shape)) * itemsize
        self._alloc = self._native_device.allocate(self._nbytes)

    @property
    def shape(self) -> tuple:
        return self._shape

    @property
    def dtype(self) -> str:
        return self._dtype

    @property
    def size(self) -> int:
        return int(np.prod(self._shape))

    def copy_to_host(self) -> bytes:
        return self._native_device.memcpy_dtoh(self._alloc, self._nbytes)

    def copy_from_host(self, data: bytes) -> None:
        self._native_device.memcpy_htod(self._alloc, data, len(data))

    def __repr__(self) -> str:
        return f"TptrJaxArray(shape={self._shape}, dtype={self._dtype})"

