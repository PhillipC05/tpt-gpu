// =============================================================================
// TPTIRDialect.h — TPTIR MLIR-Compatible Dialect Definition
// =============================================================================
// TPT GPU — Tensor Processing Technology
// License: Apache License 2.0 (with Express Patent Grant)
// =============================================================================
//
// This header defines the TPTIR dialect, providing MLIR-compatible type and
// operation abstractions for GPU kernel compilation.
//
// Design: MLIR-compatible dialect definition with C++ RAII types.
// =============================================================================

#ifndef TPTIR_DIALECT_TPTIRDIALECT_H
#define TPTIR_DIALECT_TPTIRDIALECT_H

#include <cstdint>
#include <string>
#include <vector>
#include <memory>
#include <unordered_map>

namespace tptir {

// -----------------------------------------------------------------------------
// Dialect Identifier
// -----------------------------------------------------------------------------

/// The TPTIR dialect namespace string.
constexpr const char* kDialectNamespace = "tptir";

// -----------------------------------------------------------------------------
// Address Space Enum
// -----------------------------------------------------------------------------

enum class AddressSpace : uint8_t {
  Global   = 0,
  Shared   = 1,
  Local    = 2,
  Constant = 3,
  Generic  = 4
};

const char* addressSpaceToString(AddressSpace as);
AddressSpace stringToAddressSpace(const std::string& str);
