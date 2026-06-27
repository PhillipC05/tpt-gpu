"""
TPT Framework Backends (tptr) - Python thin wrapper over Rust runtime.

Provides a Pythonic API over the TPT GPU runtime (tptr-core) via PyO3 bindings.
Includes PyTorch and JAX integration for seamless ML framework interop.

Usage:
    import tptr

    # Create a device
    device = tptr.Device(0)

    # Allocate GPU memory
    mem = device.allocate(4096)

    # Create and launch kernels
    kernel = device.create_kernel("my_kernel")
    config = tptr.KernelConfig(grid=(16, 1, 1), block=(256, 1, 1))

    # Memory operations
    device.memcpy_htod(mem, data, size)
    result = device.memcpy_dtoh(mem, size)

    # PyTorch interop
    import tptr.pytorch
    tptr.pytorch.register_backend()

    # JAX interop
    import tptr.jax
    tptr.jax.register_backend()
"""

__version__ = "0.1.0"
__license__ = "Apache-2.0"

# Re-export core types from the Rust-backed tptr module
from tptr._ffi import (  # noqa: E402
    Device,
    MemoryAllocation,
    CommandQueue,
    Kernel,
    KernelConfig,
    KernelHandle,
    TptrError,
)

# Re-export high-level wrappers
from tptr.core import (
    TptrDevice,
    TptrContext,
    TptrStream,
    TptrKernel,
    TptrMemory,
    get_device,
    get_context,
    synchronize,
)

# Re-export tensor utilities
from tptr.tensor import (
    TptrTensor,
    TptrDType,
    dtype,
    zeros,
    ones,
    empty,
    full,
)

# Re-export dispatch utilities
from tptr.dispatch import (
    DispatchRegistry,
    register_op,
    get_dispatch_table,
)

__all__ = [
    # Core types (from Rust)
    "Device",
    "MemoryAllocation",
    "CommandQueue",
    "Kernel",
    "KernelConfig",
    "KernelHandle",
    "TptrError",
    # High-level wrappers
    "TptrDevice",
    "TptrContext",
    "TptrStream",
    "TptrKernel",
    "TptrMemory",
    "get_device",
    "get_context",
    "synchronize",
    # Tensor utilities
    "TptrTensor",
    "TptrDType",
    "dtype",
    "zeros",
    "ones",
    "empty",
    "full",
    # Dispatch
    "DispatchRegistry",
    "register_op",
    "get_dispatch_table",
]

