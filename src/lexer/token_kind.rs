use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Literal(Literal),
    Punctuation(Punct),
    OpeningDelimiter(Delimiter),
    ClosingDelimiter(Delimiter),
    Keyword(Kw),
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
    And,        // '&&'
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
    Underscore, // '_'
    Pipe,       // '|'
    Or,         // '||'
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kw {
    Let,      // 'let'
    Fn,       // 'fn'
    Return,   // 'return'
    If,       // 'if'
    Else,     // 'else'
    While,    // 'while'
    Loop,     // 'loop'
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
    Use,      // 'use'
    Where,    // 'where'
    Extern,   // 'extern'
    Const,    // 'const'
    Unit,     // 'unit'
}

impl TokenKind {
    pub fn is_right_associative(&self) -> bool {
        matches!(
            self,
            TokenKind::Punctuation(Punct::Eq)
                | TokenKind::Punctuation(Punct::PlusEq)
                | TokenKind::Punctuation(Punct::MinusEq)
                | TokenKind::Punctuation(Punct::StarEq)
                | TokenKind::Punctuation(Punct::SlashEq)
                | TokenKind::Punctuation(Punct::PercentEq)
        )
    }

    pub fn is_infix_op(&self) -> bool {
        matches!(
            self,
            TokenKind::Punctuation(Punct::Plus)
                | TokenKind::Punctuation(Punct::Minus)
                | TokenKind::Punctuation(Punct::Star)
                | TokenKind::Punctuation(Punct::Slash)
                | TokenKind::Punctuation(Punct::Percent)
                | TokenKind::Punctuation(Punct::And)
                | TokenKind::Punctuation(Punct::Or)
                | TokenKind::Punctuation(Punct::Eq)
                | TokenKind::Punctuation(Punct::PlusEq)
                | TokenKind::Punctuation(Punct::MinusEq)
                | TokenKind::Punctuation(Punct::StarEq)
                | TokenKind::Punctuation(Punct::SlashEq)
                | TokenKind::Punctuation(Punct::PercentEq)
                | TokenKind::Punctuation(Punct::EqEq)
                | TokenKind::Punctuation(Punct::NotEq)
                | TokenKind::Punctuation(Punct::Less)
                | TokenKind::Punctuation(Punct::LessEq)
                | TokenKind::Punctuation(Punct::Greater)
                | TokenKind::Punctuation(Punct::GreaterEq)
        )
    }
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
            Punct::And => "&&",
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
            Punct::Underscore => "_",
            Punct::Pipe => "|",
            Punct::Or => "||",
        };
        write!(f, "{}", s)
    }
}

impl Display for Kw {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Kw::Let => "let",
            Kw::Fn => "fn",
            Kw::Return => "return",
            Kw::If => "if",
            Kw::Else => "else",
            Kw::While => "while",
            Kw::Loop => "loop",
            Kw::For => "for",
            Kw::In => "in",
            Kw::Break => "break",
            Kw::Continue => "continue",
            Kw::Struct => "struct",
            Kw::Enum => "enum",
            Kw::Trait => "trait",
            Kw::Match => "match",
            Kw::Impl => "impl",
            Kw::Pub => "pub",
            Kw::Mut => "mut",
            Kw::Type => "type",
            Kw::As => "as",
            Kw::True => "true",
            Kw::False => "false",
            Kw::Use => "use",
            Kw::Where => "where",
            Kw::Extern => "extern",
            Kw::Const => "const",
            Kw::Unit => "unit",
        };
        write!(f, "{}", s)
    }
}
