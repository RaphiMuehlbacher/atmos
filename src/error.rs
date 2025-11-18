use crate::lexer::LexerError;
use crate::parser::ParserError;
use crate::resolver::ResolverError;
use miette::Diagnostic;
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic)]
pub enum CompilerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    LexerError(#[from] LexerError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ParserError(#[from] ParserError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ResolverError(#[from] ResolverError),
}
