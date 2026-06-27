// tpt — TPT Script compiler CLI
//
// Usage:
//   tpt check   <file>              Type-check without emitting output
//   tpt compile <file> [-o <out>]   Compile to Rust + TPTIR sources
//   tpt inspect <op>                Print JSON schema for a built-in op
//   tpt run     <file>              Compile and show generated output
//   tpt ops                         List all built-in operation names
//   tpt docs    <op>                Print Markdown docs for a built-in op
//   tpt --version                   Print the compiler version

use std::{
    env,
    fs,
    process,
};

use tptb_core::{
    compile_full, compile_str,
    errors::ErrorCode,
    introspect, DocFormat,
};

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().collect();
    let exit = run(&args);
    process::exit(exit);
}

fn run(args: &[String]) -> i32 {
    match args.get(1).map(String::as_str) {
        Some("--version") | Some("-V") => {
            println!("tpt {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some("--help") | Some("-h") | None => {
            print_help();
            0
        }
        Some("check") => cmd_check(args),
        Some("compile") => cmd_compile(args),
        Some("inspect") => cmd_inspect(args),
        Some("run") => cmd_run(args),
        Some("ops") => cmd_ops(),
        Some("docs") => cmd_docs(args),
        Some(cmd) => {
            eprintln!("tpt: unknown subcommand `{cmd}`");
            eprintln!("Run `tpt --help` for usage.");
            1
        }
    }
}

// ---------------------------------------------------------------------------
// check
// ---------------------------------------------------------------------------

fn cmd_check(args: &[String]) -> i32 {
    let path = match args.get(2) {
        Some(p) => p,
        None => {
            eprintln!("tpt check: missing <file>");
            return 2;
        }
    };

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("tpt check: cannot read `{path}`: {e}");
            return 2;
        }
    };

    // Lex + parse.
    let program = match compile_str(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    // Type-check.
    let checker = tptb_core::type_check(&program);
    if checker.errors.is_empty() {
        println!("{path}: ok — 0 errors");
        return 0;
    }

    let n = checker.errors.len();
    for err in &checker.errors {
        print_error(err);
    }
    eprintln!("\n{n} error(s) found.");
    1
}

// ---------------------------------------------------------------------------
// compile
// ---------------------------------------------------------------------------

fn cmd_compile(args: &[String]) -> i32 {
    let path = match args.get(2) {
        Some(p) => p,
        None => {
            eprintln!("tpt compile: missing <file>");
            return 2;
        }
    };

    let output_arg = find_flag(args, "-o");

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("tpt compile: cannot read `{path}`: {e}");
            return 2;
        }
    };

    let (checker, output) = match compile_full(&source) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    // Errors are non-fatal for codegen but we report them.
    if !checker.errors.is_empty() {
        for err in &checker.errors {
            print_error(err);
        }
        eprintln!("\nwarning: {} type error(s) — output may be incomplete.",
                  checker.errors.len());
    }

    if let Some(out_prefix) = output_arg {
        // Write Rust source.
        let rs_path = format!("{out_prefix}.rs");
        if let Err(e) = fs::write(&rs_path, &output.rust_source) {
            eprintln!("tpt compile: cannot write `{rs_path}`: {e}");
            return 2;
        }
        // Write TPTIR source (if any GPU kernels were emitted).
        if !output.tptir_source.is_empty() {
            let ir_path = format!("{out_prefix}.tptir");
            if let Err(e) = fs::write(&ir_path, &output.tptir_source) {
                eprintln!("tpt compile: cannot write `{ir_path}`: {e}");
                return 2;
            }
            println!("compiled: {rs_path}  {ir_path}");
        } else {
            println!("compiled: {rs_path}");
        }
    } else {
        // Default: print to stdout.
        println!("// --- Rust output ---");
        println!("{}", output.rust_source);
        if !output.tptir_source.is_empty() {
            println!("// --- TPTIR output ---");
            println!("{}", output.tptir_source);
        }
    }
    0
}

// ---------------------------------------------------------------------------
// inspect
// ---------------------------------------------------------------------------

fn cmd_inspect(args: &[String]) -> i32 {
    let op_name = match args.get(2) {
        Some(n) => n.as_str(),
        None => {
            eprintln!("tpt inspect: missing <op>");
            eprintln!("  Tip: run `tpt ops` to list available operations.");
            return 2;
        }
    };

    match introspect::get_schema(op_name) {
        Some(schema) => {
            println!("{}", introspect::schema_to_json(schema));
            0
        }
        None => {
            eprintln!("tpt inspect: unknown operation `{op_name}`");
            eprintln!("  Tip: run `tpt ops` to list available operations.");
            1
        }
    }
}

// ---------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------

fn cmd_run(args: &[String]) -> i32 {
    let path = match args.get(2) {
        Some(p) => p,
        None => {
            eprintln!("tpt run: missing <file>");
            return 2;
        }
    };

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("tpt run: cannot read `{path}`: {e}");
            return 2;
        }
    };

    let (checker, output) = match compile_full(&source) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    if !checker.errors.is_empty() {
        for err in &checker.errors {
            print_error(err);
        }
        eprintln!("\n{} error(s). Refusing to run.", checker.errors.len());
        return 1;
    }

    println!("// Rust source");
    println!("{}", output.rust_source);
    if !output.tptir_source.is_empty() {
        println!("// TPTIR source");
        println!("{}", output.tptir_source);
    }
    0
}

// ---------------------------------------------------------------------------
// ops
// ---------------------------------------------------------------------------

fn cmd_ops() -> i32 {
    let names = introspect::list_operations();
    for name in &names {
        println!("{name}");
    }
    println!("\n{} operations total.", names.len());
    0
}

// ---------------------------------------------------------------------------
// docs
// ---------------------------------------------------------------------------

fn cmd_docs(args: &[String]) -> i32 {
    let op_name = match args.get(2) {
        Some(n) => n.as_str(),
        None => {
            eprintln!("tpt docs: missing <op>");
            return 2;
        }
    };

    let fmt = match args.get(3).map(String::as_str) {
        Some("pyi") => DocFormat::Pyi,
        _           => DocFormat::Markdown,
    };
    let doc = introspect::generate_docs(op_name, fmt);
    if doc.is_empty() {
        eprintln!("tpt docs: no documentation for `{op_name}`");
        return 1;
    }
    println!("{doc}");
    0
}

// ---------------------------------------------------------------------------
// Error rendering
// ---------------------------------------------------------------------------

fn print_error(err: &tptb_core::errors::TptError) {
    // Headline
    eprintln!("error[{}] {}:{} — {}",
              err.code.as_str(),
              err.span.line, err.span.col,
              err.message);

    // Structured context fields
    for (k, v) in err.context.fields() {
        eprintln!("  {k}: {v}");
    }

    // Fix code takes priority over suggestion
    if let Some(fix) = &err.fix_code {
        eprintln!("  fix: {fix}");
    } else if let Some(sug) = &err.suggestion {
        eprintln!("  suggestion: {sug}");
    }

    // Extra hint for common codes
    match &err.code {
        ErrorCode::ShapeMismatch => {
            eprintln!("  hint: use tpt.reshape() or tpt.broadcast_to() to adjust shapes");
        }
        ErrorCode::DtypeMismatch => {
            eprintln!("  hint: use tpt.cast(x, dtype=...) for explicit dtype conversion");
        }
        ErrorCode::UndefinedVariable => {
            eprintln!("  hint: check spelling or move the declaration before first use");
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_flag<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].as_str())
}

fn print_help() {
    println!("tpt {} — TPT Script compiler", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("  tpt <SUBCOMMAND> [OPTIONS]");
    println!();
    println!("SUBCOMMANDS:");
    println!("  check   <file>              Type-check a .tpts file");
    println!("  compile <file> [-o <out>]   Compile to Rust + TPTIR");
    println!("  inspect <op>                Show JSON schema for a built-in op");
    println!("  run     <file>              Compile and print generated output");
    println!("  ops                         List all built-in operation names");
    println!("  docs    <op> [markdown|pyi] Show documentation for a built-in op");
    println!();
    println!("FLAGS:");
    println!("  -h, --help      Print this help message");
    println!("  -V, --version   Print the compiler version");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_version_flag() {
        assert_eq!(run(&args(&["tpt", "--version"])), 0);
    }

    #[test]
    fn test_help_flag() {
        assert_eq!(run(&args(&["tpt", "--help"])), 0);
    }

    #[test]
    fn test_no_args_shows_help() {
        assert_eq!(run(&args(&["tpt"])), 0);
    }

    #[test]
    fn test_unknown_subcommand() {
        assert_eq!(run(&args(&["tpt", "frobnicate"])), 1);
    }

    #[test]
    fn test_ops_lists_operations() {
        assert_eq!(run(&args(&["tpt", "ops"])), 0);
    }

    #[test]
    fn test_inspect_known_op() {
        assert_eq!(run(&args(&["tpt", "inspect", "matmul"])), 0);
    }

    #[test]
    fn test_inspect_unknown_op() {
        assert_eq!(run(&args(&["tpt", "inspect", "nonexistent_op_xyz"])), 1);
    }

    #[test]
    fn test_docs_known_op() {
        assert_eq!(run(&args(&["tpt", "docs", "matmul"])), 0);
    }

    #[test]
    fn test_check_missing_file_arg() {
        assert_eq!(run(&args(&["tpt", "check"])), 2);
    }

    #[test]
    fn test_compile_missing_file_arg() {
        assert_eq!(run(&args(&["tpt", "compile"])), 2);
    }

    #[test]
    fn test_run_missing_file_arg() {
        assert_eq!(run(&args(&["tpt", "run"])), 2);
    }

    #[test]
    fn test_inspect_missing_op_arg() {
        assert_eq!(run(&args(&["tpt", "inspect"])), 2);
    }

    #[test]
    fn test_docs_missing_op_arg() {
        assert_eq!(run(&args(&["tpt", "docs"])), 2);
    }

    #[test]
    fn test_find_flag_present() {
        let a = args(&["tpt", "compile", "foo.tpts", "-o", "out"]);
        assert_eq!(find_flag(&a, "-o"), Some("out"));
    }

    #[test]
    fn test_find_flag_absent() {
        let a = args(&["tpt", "compile", "foo.tpts"]);
        assert_eq!(find_flag(&a, "-o"), None);
    }

    // Round-trip: check a valid .tpts source string via the core API.
    #[test]
    fn test_check_valid_source_via_api() {
        let source = r#"
fn add(a: f32, b: f32) -> f32 {
    return a + b
}
"#;
        let program = compile_str(source).expect("parse failed");
        let checker = tptb_core::type_check(&program);
        assert!(checker.errors.is_empty(), "{:?}", checker.errors);
    }

    // Round-trip: check that a type error in source is detected.
    #[test]
    fn test_check_bad_source_via_api() {
        let source = r#"
fn f() -> f32 {
    return true
}
"#;
        let program = compile_str(source).expect("parse failed");
        let checker = tptb_core::type_check(&program);
        assert!(!checker.errors.is_empty());
    }

    // Round-trip: compile a simple function end-to-end.
    #[test]
    fn test_compile_simple_fn_via_api() {
        let source = r#"
import tpt
fn relu_add(x: Tensor[f32, m, n], y: Tensor[f32, m, n]) -> Tensor[f32, m, n] {
    let r = tpt.relu(x)
    let s = r + y
    return s
}
"#;
        let (checker, output) = compile_full(source).expect("compile_full failed");
        assert!(checker.errors.is_empty(), "{:?}", checker.errors);
        assert!(!output.rust_source.is_empty());
    }
}
