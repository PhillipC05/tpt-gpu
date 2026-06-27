pub mod ast;
pub mod lexer;
pub mod parser;
pub mod semantic;

pub use lexer::{tokenize, LexError, Span, Token, TokenKind};
pub use parser::{parse, ParseError};
pub use ast::Program;
pub use semantic::{type_check, TypeChecker};

/// Convenience: lex and parse a TPT Script source string in one call.
pub fn compile_str(source: &str) -> Result<Program, CompileError> {
    let tokens = tokenize(source)?;
    let program = parse(tokens)?;
    Ok(program)
}

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("lex error: {0}")]
    Lex(#[from] LexError),
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
}
