use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Literal(Literal),
    Punctuation(Punct),
    OpeningDelimiter(Delimiter),
    ClosingDelimiter(Delimiter),
    Keyword(Keyword),
    EOF,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    I32(i32),
    U32(u32),
    F64(f64),
    Str(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Delimiter {
    Paren,   // '(' ')'
    Bracket, // '[' ']'
    Brace,   // '{' '}'
}

#[derive(Clone, Debug, PartialEq)]
pub enum Punct {
    Plus,       // '+'
    Minus,      // '-'
    Star,       // '*'
    Slash,      // '/'
    Percent,    // '%'
    Ampersand,  // '&'
    PlusEq,     // '+='
    MinusEq,    // '-='
    StarEq,     // '*='
    SlashEq,    // '/='
    PercentEq,  // '%='
    Bang,       // '!'
    Eq,         // '='
    EqEq,       // '=='
    NotEq,      // '!='
    Less,       // '<'
    LessEq,     // '<='
    Greater,    // '>'
    GreaterEq,  // '>='
    Arrow,      // '->'
    FatArrow,   // '=>'
    Dot,        // '.'
    Semicolon,  // ';'
    Comma,      // ','
    Question,   // '?'
    Colon,      // ':'
    ColonColon, // '::'
}

#[derive(Clone, Debug, PartialEq)]
pub enum Keyword {
    Let,      // 'let'
    Fn,       // 'fn'
    Return,   // 'return'
    If,       // 'if'
    Else,     // 'else'
    While,    // 'while'
    For,      // 'for'
    In,       // 'in'
    Break,    // 'break'
    Continue, // 'continue'
    Struct,   // 'struct'
    Enum,     // 'enum'
    Trait,    // 'trait'
    Match,    // 'match'
    Impl,     // 'impl'
    Pub,      // 'pub'
    Mut,      // 'mut'
    Type,     // 'type'
    As,       // 'as'
    True,     // 'true'
    False,    // 'false'
    Null,     // 'null'
    SelfKw,   // 'self'
    Super,    // 'super'
    Use,      // 'use'
    Where,    // 'where'
    Extern,   // 'extern'
    Const,    // 'const'
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Literal(lit) => write!(f, "{}", lit),
            TokenKind::Punctuation(p) => write!(f, "{}", p),
            TokenKind::OpeningDelimiter(d) => write!(f, "{}", d),
            TokenKind::ClosingDelimiter(d) => match d {
                Delimiter::Paren => write!(f, ")"),
                Delimiter::Bracket => write!(f, "]"),
                Delimiter::Brace => write!(f, "}}"),
            },
            TokenKind::Keyword(k) => write!(f, "{}", k),
            TokenKind::EOF => write!(f, "EOF"),
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Literal::I32(n) => write!(f, "{}i32", n),
            Literal::U32(n) => write!(f, "{}u32", n),
            Literal::F64(n) => write!(f, "{}f64", n),
            Literal::Str(s) => write!(f, "\"{}\"", s),
        }
    }
}

impl Display for Delimiter {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Delimiter::Paren => write!(f, "("),
            Delimiter::Bracket => write!(f, "["),
            Delimiter::Brace => write!(f, "{{"),
        }
    }
}

impl Display for Punct {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Punct::Plus => "+",
            Punct::Minus => "-",
            Punct::Star => "*",
            Punct::Slash => "/",
            Punct::Percent => "%",
            Punct::Ampersand => "&",
            Punct::PlusEq => "+=",
            Punct::MinusEq => "-=",
            Punct::StarEq => "*=",
            Punct::SlashEq => "/=",
            Punct::PercentEq => "%=",
            Punct::Bang => "!",
            Punct::Eq => "=",
            Punct::EqEq => "==",
            Punct::NotEq => "!=",
            Punct::Less => "<",
            Punct::LessEq => "<=",
            Punct::Greater => ">",
            Punct::GreaterEq => ">=",
            Punct::Arrow => "->",
            Punct::FatArrow => "=>",
            Punct::Dot => ".",
            Punct::Semicolon => ";",
            Punct::Comma => ",",
            Punct::Question => "?",
            Punct::Colon => ":",
            Punct::ColonColon => "::",
        };
        write!(f, "{}", s)
    }
}

impl Display for Keyword {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Keyword::Let => "let",
            Keyword::Fn => "fn",
            Keyword::Return => "return",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::For => "for",
            Keyword::In => "in",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
            Keyword::Trait => "trait",
            Keyword::Match => "match",
            Keyword::Impl => "impl",
            Keyword::Pub => "pub",
            Keyword::Mut => "mut",
            Keyword::Type => "type",
            Keyword::As => "as",
            Keyword::True => "true",
            Keyword::False => "false",
            Keyword::Null => "null",
            Keyword::SelfKw => "self",
            Keyword::Super => "super",
            Keyword::Use => "use",
            Keyword::Where => "where",
            Keyword::Extern => "extern",
            Keyword::Const => "const",
        };
        write!(f, "{}", s)
    }
}
