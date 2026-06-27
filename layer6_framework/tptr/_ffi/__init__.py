"""
FFI bridge to the Rust tptr library.

This module provides direct access to the PyO3-backed tptr extension.
When the native extension is not available, a simulation fallback is provided.
"""

import importlib
import os
import sys
import warnings

# Try to import the native Rust extension
_native_ext = None
try:
    import tptr as _native_ext  # type: ignore
except ImportError:
    try:
        # Try loading from a pre-built path
        _ext_path = os.path.join(os.path.dirname(__file__), "..", "..", "target", "release")
        if os.path.exists(_ext_path):
            sys.path.insert(0, _ext_path)
            import tptr as _native_ext  # type: ignore
    except ImportError:
        pass

if _native_ext is not None:
    # Re-export all native types
    Device = _native_ext.Device
    MemoryAllocation = _native_ext.MemoryAllocation
    CommandQueue = _native_ext.CommandQueue
    Kernel = _native_ext.Kernel
    KernelConfig = _native_ext.KernelConfig
    KernelHandle = _native_ext.KernelHandle
    TptrError = _native_ext.TptrError
else:
    # Simulation fallback for development/testing without native extension
    warnings.warn(
        "Native tptr extension not found. Using simulation fallback. "
        "Build with: cd layer4_tptr && cargo build -p tptr-py",
        RuntimeWarning,
        stacklevel=2,
    )
    from ._sim import (
        Device,
        MemoryAllocation,
        CommandQueue,
        Kernel,
        KernelConfig,
        KernelHandle,
        TptrError,
    )

