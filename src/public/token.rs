use crate::lexer::token_kind::{Delimiter, Kw, Literal, Punct, TokenKind};
use serde::{Deserialize, Serialize};

use super::span::Span;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PublicTokenKind {
    Ident(String),
    Literal(PublicLiteral),
    Punctuation(PunctuationKind),
    OpeningDelimiter(DelimiterKind),
    ClosingDelimiter(DelimiterKind),
    Keyword(KeywordKind),
    EOF,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PublicLiteral {
    Integer { value: String, suffix: Option<String> },
    Float { value: String, suffix: Option<String> },
    Str(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PunctuationKind {
    Plus, Minus, Star, Slash, Percent, Ampersand, And,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    Bang, Eq, EqEq, NotEq, Less, LessEq, Greater, GreaterEq,
    Arrow, FatArrow, Dot, Semicolon, Comma, Question, Colon, ColonColon,
    Underscore, Pipe, Or,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DelimiterKind {
    Paren, Bracket, Brace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum KeywordKind {
    Let, Fn, Return, If, Else, While, Loop, For, In,
    Break, Continue, Struct, Enum, Trait, Mod, Match,
    Impl, Pub, Mut, Type, As, True, False, Use, Where, Extern, Const,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicToken {
    pub kind: PublicTokenKind,
    pub span: Span,
}

fn map_literal(lit: &Literal) -> PublicLiteral {
    match lit {
        Literal::Integer { value, suffix } => PublicLiteral::Integer {
            value: value.clone(),
            suffix: suffix.clone(),
        },
        Literal::Float { value, suffix } => PublicLiteral::Float {
            value: value.clone(),
            suffix: suffix.clone(),
        },
        Literal::Str(s) => PublicLiteral::Str(s.clone()),
    }
}

impl From<&TokenKind> for PublicTokenKind {
    fn from(kind: &TokenKind) -> Self {
        match kind {
            TokenKind::Ident(s) => PublicTokenKind::Ident(s.clone()),
            TokenKind::Literal(lit) => PublicTokenKind::Literal(map_literal(lit)),
            TokenKind::Punctuation(p) => PublicTokenKind::Punctuation(match p {
                Punct::Plus => PunctuationKind::Plus,
                Punct::Minus => PunctuationKind::Minus,
                Punct::Star => PunctuationKind::Star,
                Punct::Slash => PunctuationKind::Slash,
                Punct::Percent => PunctuationKind::Percent,
                Punct::Ampersand => PunctuationKind::Ampersand,
                Punct::And => PunctuationKind::And,
                Punct::PlusEq => PunctuationKind::PlusEq,
                Punct::MinusEq => PunctuationKind::MinusEq,
                Punct::StarEq => PunctuationKind::StarEq,
                Punct::SlashEq => PunctuationKind::SlashEq,
                Punct::PercentEq => PunctuationKind::PercentEq,
                Punct::Bang => PunctuationKind::Bang,
                Punct::Eq => PunctuationKind::Eq,
                Punct::EqEq => PunctuationKind::EqEq,
                Punct::NotEq => PunctuationKind::NotEq,
                Punct::Less => PunctuationKind::Less,
                Punct::LessEq => PunctuationKind::LessEq,
                Punct::Greater => PunctuationKind::Greater,
                Punct::GreaterEq => PunctuationKind::GreaterEq,
                Punct::Arrow => PunctuationKind::Arrow,
                Punct::FatArrow => PunctuationKind::FatArrow,
                Punct::Dot => PunctuationKind::Dot,
                Punct::Semicolon => PunctuationKind::Semicolon,
                Punct::Comma => PunctuationKind::Comma,
                Punct::Question => PunctuationKind::Question,
                Punct::Colon => PunctuationKind::Colon,
                Punct::ColonColon => PunctuationKind::ColonColon,
                Punct::Underscore => PunctuationKind::Underscore,
                Punct::Pipe => PunctuationKind::Pipe,
                Punct::Or => PunctuationKind::Or,
            }),
            TokenKind::OpeningDelimiter(d) => PublicTokenKind::OpeningDelimiter(match d {
                Delimiter::Paren => DelimiterKind::Paren,
                Delimiter::Bracket => DelimiterKind::Bracket,
                Delimiter::Brace => DelimiterKind::Brace,
            }),
            TokenKind::ClosingDelimiter(d) => PublicTokenKind::ClosingDelimiter(match d {
                Delimiter::Paren => DelimiterKind::Paren,
                Delimiter::Bracket => DelimiterKind::Bracket,
                Delimiter::Brace => DelimiterKind::Brace,
            }),
            TokenKind::Keyword(k) => PublicTokenKind::Keyword(match k {
                Kw::Let => KeywordKind::Let,
                Kw::Fn => KeywordKind::Fn,
                Kw::Return => KeywordKind::Return,
                Kw::If => KeywordKind::If,
                Kw::Else => KeywordKind::Else,
                Kw::While => KeywordKind::While,
                Kw::Loop => KeywordKind::Loop,
                Kw::For => KeywordKind::For,
                Kw::In => KeywordKind::In,
                Kw::Break => KeywordKind::Break,
                Kw::Continue => KeywordKind::Continue,
                Kw::Struct => KeywordKind::Struct,
                Kw::Enum => KeywordKind::Enum,
                Kw::Trait => KeywordKind::Trait,
                Kw::Mod => KeywordKind::Mod,
                Kw::Match => KeywordKind::Match,
                Kw::Impl => KeywordKind::Impl,
                Kw::Pub => KeywordKind::Pub,
                Kw::Mut => KeywordKind::Mut,
                Kw::Type => KeywordKind::Type,
                Kw::As => KeywordKind::As,
                Kw::True => KeywordKind::True,
                Kw::False => KeywordKind::False,
                Kw::Use => KeywordKind::Use,
                Kw::Where => KeywordKind::Where,
                Kw::Extern => KeywordKind::Extern,
                Kw::Const => KeywordKind::Const,
            }),
            TokenKind::EOF => PublicTokenKind::EOF,
        }
    }
}
