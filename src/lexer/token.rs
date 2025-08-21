use crate::lexer::TokenKind;
use miette::SourceSpan;
use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    kind: TokenKind,
    span: SourceSpan,
}

impl Token {
    pub fn new(kind: TokenKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{} [offset: {}, length: {}]",
            self.kind,
            self.span.offset(),
            self.span.len()
        )
    }
}
