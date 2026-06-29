//! TPTIR Dialect Specification
//!
//! This crate is the canonical, versioned definition of the TPTIR intermediate
//! representation shared across the TPT compute suite (tpt-gpu, tpt-crucible,
//! tpt-spark). Any tool that reads or writes TPTIR text should depend on this
//! crate rather than re-defining the dialect inline.
//!
//! # Stability
//! Types in this crate are stable from v0.1.0 onward. Additive changes increment
//! the minor version; breaking changes increment the major version.

pub mod types;
pub mod ops;
pub mod attr;
pub mod text;
pub mod dialect;

pub use types::{Type, TypeKind, AddressSpace, ElemType};
pub use ops::{Op, OpDef, OpCategory};
pub use attr::{Attr, AttrValue};
pub use dialect::{Dialect, DialectRegistry, TPTIR_DIALECT_VERSION};
pub use text::{emit, parse, EmitOptions};
