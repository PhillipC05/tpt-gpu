"""
TPT PyTorch Dispatch Layer.

Integrates TPT GPU runtime as a PyTorch backend device,
allowing PyTorch tensors to use TPT for computation via
the private-use-one device mechanism.

Usage:
    import torch
    import tptr.pytorch

    # Register TPT as a backend
    tptr.pytorch.register_backend()

    # Create a tensor on TPT device
    x = torch.randn(32, 64, device="tpt:0")

    # Operations are dispatched to TPT runtime
    y = x @ x.T  # matmul via TPT
    z = torch.relu(y) # activation via TPT
"""
from __future__ import annotations
from typing import Optional, Dict, Any, Tuple, List, Union
import os
import warnings

from tptr._ffi import TptrError


# TPT device type string for PyTorch
TPT_DEVICE_TYPE = "tpt"


def is_available() -> bool:
    """Check if TPT backend is available."""
    try:
        import tptr._ffi
        return True
    except ImportError:
        return False


def register_backend() -> bool:
    """
    Register TPT as a PyTorch backend.

    This registers the TPT device with PyTorch's dispatch system,
    allowing tensor operations to be routed through TPT.

    Returns True if registration succeeded.
    """
    try:
        import torch
    except ImportError:
        warnings.warn("PyTorch not installed. Cannot register TPT backend.")
        return False

    try:
        # Register the device with PyTorch
        if hasattr(torch, "_register_device_backend"):
            torch._register_device_backend(TPT_DEVICE_TYPE, _dispatch_impl)
        return True
    except Exception as e:
        warnings.warn(f"Failed to register TPT backend: {e}")
        return False


def _dispatch_impl(op, args, kwargs):
    """Dispatch a PyTorch operation to TPT runtime."""
    from tptr.pytorch.ops import dispatch_op
    return dispatch_op(op, args, kwargs)


def get_tpt_device(device_str: str) -> "TptrTorchDevice":
    """Parse a TPT device string like 'tpt:0' and return a device object."""
    if ":" in device_str:
        _, idx = device_str.split(":", 1)
        index = int(idx)
    else:
        index = 0
    return TptrTorchDevice(index)


class TptrTorchDevice:
    """PyTorch-compatible device wrapper for TPT."""

    def __init__(self, index: int = 0):
        from tptr._ffi import Device as NativeDevice
        self._index = index
        self._device = NativeDevice(index)

    @property
    def index(self) -> int:
        return self._index

    @property
    def name(self) -> str:
        info = self._device.info()
        return info.get("name", f"TPT Device {self._index}")

    @property
    def total_memory(self) -> int:
        info = self._device.info()
        return int(info.get("total_memory", 0))

    def allocate(self, size: int) -> "TptrNativeTensor":
        alloc = self._device.allocate(size)
        return TptrNativeTensor(alloc)

    def synchronize(self) -> None:
        self._device.synchronize()

    def __repr__(self) -> str:
        return f"tptr:{self._index}"


class TptrNativeTensor:
    """Wrapper around native TPT memory allocation for PyTorch interop."""

    def __init__(self, alloc):
        self._alloc = alloc

    @property
    def handle(self) -> int:
        return self._alloc.handle

    @property
    def size(self) -> int:
        return self._alloc.size

    @property
    def device_ptr(self) -> int:
        return self._alloc.device_ptr

    def copy_to_host(self, size: int) -> bytes:
        from tptr._ffi import Device as NativeDevice
        dev = NativeDevice(0)
        return dev.memcpy_dtoh(self._alloc, size)

    def copy_from_host(self, data: bytes, size: int) -> None:
        from tptr._ffi import Device as NativeDevice
        dev = NativeDevice(0)
        dev.memcpy_htod(self._alloc, data, size)

