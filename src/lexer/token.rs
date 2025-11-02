use crate::lexer::token_kind::{Kw, Punct};
use crate::lexer::TokenKind;
use miette::SourceSpan;
use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

impl Token {
    pub fn new(kind: TokenKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }
    pub fn can_begin_expr(&self) -> bool {
        match self.kind {
            TokenKind::Literal(_)
            | TokenKind::Keyword(_)
            | TokenKind::Punctuation(Punct::Bang)
            | TokenKind::Punctuation(Punct::Minus)
            | TokenKind::Punctuation(Punct::Star)
            | TokenKind::OpeningDelimiter(_)
            | TokenKind::Punctuation(Punct::Ampersand) => true,
            _ => false,
        }
    }

    pub fn begins_item(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Keyword(Kw::Fn)
                | TokenKind::Keyword(Kw::Struct)
                | TokenKind::Keyword(Kw::Enum)
                | TokenKind::Keyword(Kw::Impl)
                | TokenKind::Keyword(Kw::Trait)
                | TokenKind::Keyword(Kw::Extern)
                | TokenKind::Keyword(Kw::Const)
                | TokenKind::Keyword(Kw::Use)
                | TokenKind::Keyword(Kw::Type)
        )
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
