use crate::lexer::token_kind::{Keyword, Punct};
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
            TokenKind::Keyword(Keyword::Fn)
                | TokenKind::Keyword(Keyword::Struct)
                | TokenKind::Keyword(Keyword::Enum)
                | TokenKind::Keyword(Keyword::Impl)
                | TokenKind::Keyword(Keyword::Trait)
                | TokenKind::Keyword(Keyword::Extern)
                | TokenKind::Keyword(Keyword::Const)
                | TokenKind::Keyword(Keyword::Use)
                | TokenKind::Keyword(Keyword::Type)
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
