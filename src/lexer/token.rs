use crate::lexer::TokenKind;
use crate::lexer::token_kind::{Kw, Punct};
use miette::SourceSpan;
use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

impl Token {
    #[must_use]
    pub fn new(kind: TokenKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }

    #[must_use]
    pub fn can_begin_expr(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Literal(_)
                | TokenKind::Keyword(_)
                | TokenKind::Punctuation(Punct::Bang)
                | TokenKind::Punctuation(Punct::Minus)
                | TokenKind::Punctuation(Punct::Star)
                | TokenKind::OpeningDelimiter(_)
                | TokenKind::Punctuation(Punct::Ampersand)
        )
    }

    #[must_use]
    pub fn begins_item(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Keyword(
                Kw::Fn | Kw::Struct | Kw::Enum | Kw::Impl | Kw::Trait | Kw::Extern | Kw::Const | Kw::Use | Kw::Type
            )
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
