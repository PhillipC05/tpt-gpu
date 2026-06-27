#!/usr/bin/env python3
import os
BASE = r"d:\Programming\1PRODUCTION\Open Source\tpt-gpu\layer3_tptc"
def w(p, c):
    full = os.path.join(BASE, p)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, 'w', encoding='utf-8', newline='\n') as f:
        f.write(c.lstrip('\n'))
    print(f"  {p}")

w("rust/Cargo.toml", """[package]
name = "tptc-rs"
version = "0.1.0"
edition = "2021"
description = "Rust port of TPTIR compiler stack"
[lib]
name = "tptc_rs"
crate-type = ["lib", "cdylib"]
[dependencies]
libc = "0.2"
[features]
default = ["ffi"]
ffi = []
""")

w("rust/src/lib.rs", """pub mod ffi;
pub mod ir;
pub mod passes;
pub const VERSION: &str = "0.1.0";
pub fn compile(source: &str, target: &str) -> Result<String, String> {
    #[cfg(feature = "ffi")] { ffi::compile_via_ffi(source, target) }
    #[cfg(not(feature = "ffi"))] { compile_native(source, target) }
}
pub fn compile_native(source: &str, target: &str) -> Result<String, String> {
    let region = ir::parse_assembly(source)?;
    let passes = passes::default_pipeline();
    let _changes = passes.run(&region);
    match target {
        "tptisa" | "text" => Ok(region.to_string()),
        "llvmir" => Ok(generate_llvm_ir(&region)),
        _ => Err(format!("Unknown target: {}", target)),
    }
}
fn generate_llvm_ir(region: &ir::Region) -> String {
    let mut out = String::from("; LLVM IR\\ndefine void @kernel() {\\n");
    for block in &region.blocks {
        out.push_str(&format!("  {}:\\n", block.label));
        for op in &block.operations {
            out.push_str(&format!("    {}\\n", op.display()));
        }
    }
    out.push_str("}\\n"); out
}
pub fn version() -> String { format!("tptc-rs v{}", VERSION) }
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_version() { assert!(version().contains("0.1.0")); }
    #[test] fn test_ir_types() { let t = ir::Type::primitive("i32"); assert_eq!(t.to_string(), "i32"); }
    #[test] fn test_block() { let b = ir::Block::new("entry"); assert_eq!(b.label, "entry"); }
}
""")

w("rust/src/ffi.rs", """use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
#[repr(C)]
pub enum TptirStatus { Ok = 0, ErrorGeneric = -1, ErrorParse = -2, ErrorNullPointer = -7 }
#[repr(C)]
pub struct TptirString { pub data: *mut c_char, pub size: usize }
#[repr(C)]
pub struct TptirVersion { pub major: u32, pub minor: u32, pub patch: u32 }
extern "C" {
    fn tptir_init(ctx: *mut *mut c_void) -> c_int;
    fn tptir_shutdown(ctx: *mut c_void) -> c_int;
    fn tptir_get_version() -> TptirVersion;
    fn tptir_compile(source: *const c_char, len: usize, target: c_int, out: *mut TptirString, err: *mut TptirString) -> c_int;
    fn tptir_string_free(s: *mut TptirString);
}
pub fn compile_via_ffi(source: &str, target: &str) -> Result<String, String> {
    let tid = match target { "tptisa" => 0i32, "llvmir" => 1i32, "text" => 2i32, _ => return Err(format!("Unknown target: {}", target)), };
    let csrc = CString::new(source).map_err(|e| format!("Invalid source: {}", e))?;
    unsafe {
        let mut out = TptirString { data: std::ptr::null_mut(), size: 0 };
        let mut err = TptirString { data: std::ptr::null_mut(), size: 0 };
        let status = tptir_compile(csrc.as_ptr(), source.len(), tid, &mut out, &mut err);
        if status == 0 && !out.data.is_null() {
            let result = CStr::from_ptr(out.data).to_string_lossy().into_owned();
            tptir_string_free(&mut out); Ok(result)
        } else {
            let msg = if !err.data.is_null() { CStr::from_ptr(err.data).to_string_lossy().into_owned() } else { "FFI error".into() };
            tptir_string_free(&mut err); Err(msg)
        }
    }
}
#[cfg(test)]
mod tests { use super::*;
    #[test] fn test_ffi_version() { unsafe { let v = tptir_get_version(); assert!(v.major == 0); } }
}
""")
print("Rust batch 1 done!")
