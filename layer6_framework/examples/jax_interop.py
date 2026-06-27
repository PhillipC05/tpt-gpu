#!/usr/bin/env python3
"""
JAX interop example for tptr framework backends.

Demonstrates:
- Registering TPT as a JAX backend
- Creating JAX-compatible arrays
- Backend name registration

Note: This example requires JAX to be installed.
"""
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import tptr.jax


def main():
    print("=" * 60)
    print("TPT Framework Backends - JAX Interop Example")
    print("=" * 60)

    # 1. Check availability
    print("\n1. Checking JAX availability")
    available = tptr.jax.is_available()
    print(f"   JAX available: {available}")

    # 2. Register backend
    print("\n2. Registering TPT as JAX backend")
    success = tptr.jax.register_backend()
    print(f"   Registration: {'success' if success else 'failed (JAX not installed)'}")

    # 3. Backend name
    print("\n3. Backend information")
    backend_name = tptr.jax.get_backend_name()
    print(f"   Backend name: {backend_name}")

    # 4. Create JAX-compatible array (if JAX is available)
    if available:
        print("\n4. Creating JAX-compatible arrays")
        try:
            arr = tptr.jax.TptrJaxArray((32, 64), "float32")
            print(f"   Array: {arr}")
            print(f"   Shape: {arr.shape}")
            print(f"   Dtype: {arr.dtype}")
            print(f"   Size: {arr.size}")
        except Exception as e:
            print(f"   Note: {e}")

    print("\n" + "=" * 60)
    print("JAX interop example completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()

