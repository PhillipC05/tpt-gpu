#!/usr/bin/env python3
"""
PyTorch interop example for tptr framework backends.

Demonstrates:
- Registering TPT as a PyTorch backend
- Checking supported operations
- Device creation and memory management

Note: This example requires PyTorch to be installed.
"""
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import tptr.pytorch


def main():
    print("=" * 60)
    print("TPT Framework Backends - PyTorch Interop Example")
    print("=" * 60)

    # 1. Check availability
    print("\n1. Checking TPT availability")
    available = tptr.pytorch.is_available()
    print(f"   TPT available: {available}")

    # 2. Register backend
    print("\n2. Registering TPT backend")
    success = tptr.pytorch.register_backend()
    print(f"   Registration: {'success' if success else 'failed (PyTorch not installed)'}")

    # 3. List supported ops
    print("\n3. Supported PyTorch operations")
    supported = tptr.pytorch.ops.get_supported_ops()
    for op in supported[:5]:
        tpt_op = tptr.pytorch.ops.get_tpt_op_name(op)
        print(f"   {op} -> {tpt_op}")
    print(f"   ... and {len(supported) - 5} more")

    # 4. Check specific ops
    print("\n4. Checking specific operations")
    test_ops = ["aten.add.Tensor", "aten.relu.default", "aten.mm.default", "aten.foo.bar"]
    for op in test_ops:
        supported = tptr.pytorch.ops.is_supported(op)
        print(f"   {op}: {'supported' if supported else 'not supported'}")

    # 5. TPT device info
    print("\n5. TPT Device Information")
    from tptr.pytorch import TptrTorchDevice
    dev = TptrTorchDevice(0)
    print(f"   Device: {dev}")
    print(f"   Name: {dev.name}")
    print(f"   Memory: {dev.total_memory / (1024**3):.1f} GB")

    print("\n" + "=" * 60)
    print("PyTorch interop example completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()

