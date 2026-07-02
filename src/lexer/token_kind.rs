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
    Integer { value: String, suffix: Option<String> },
    Float { value: String, suffix: Option<String> },
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
    Mod,      // 'mod'
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
}

impl TokenKind {
    #[must_use]
    pub fn is_right_associative(&self) -> bool {
        matches!(
            self,
            Self::Punctuation(
                Punct::Eq | Punct::PlusEq | Punct::MinusEq | Punct::StarEq | Punct::SlashEq | Punct::PercentEq
            )
        )
    }

    #[must_use]
    pub fn is_infix_op(&self) -> bool {
        matches!(
            self,
            Self::Punctuation(
                Punct::Plus
                    | Punct::Minus
                    | Punct::Star
                    | Punct::Slash
                    | Punct::Percent
                    | Punct::And
                    | Punct::Or
                    | Punct::Eq
                    | Punct::PlusEq
                    | Punct::MinusEq
                    | Punct::StarEq
                    | Punct::SlashEq
                    | Punct::PercentEq
                    | Punct::EqEq
                    | Punct::NotEq
                    | Punct::Less
                    | Punct::LessEq
                    | Punct::Greater
                    | Punct::GreaterEq
            )
        )
    }
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Ident(s) => write!(f, "{s}"),
            Self::Literal(lit) => write!(f, "{lit}"),
            Self::Punctuation(p) => write!(f, "{p}"),
            Self::OpeningDelimiter(d) => write!(f, "{d}"),
            Self::ClosingDelimiter(d) => match d {
                Delimiter::Paren => write!(f, ")"),
                Delimiter::Bracket => write!(f, "]"),
                Delimiter::Brace => write!(f, "}}"),
            },
            Self::Keyword(k) => write!(f, "{k}"),
            Self::EOF => write!(f, "EOF"),
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Integer { value, suffix } | Self::Float { value, suffix } => match suffix {
                Some(s) => write!(f, "{value}{s}"),
                None => write!(f, "{value}"),
            },
            Self::Str(s) => write!(f, "\"{s}\""),
        }
    }
}

impl Display for Delimiter {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Paren => write!(f, "("),
            Self::Bracket => write!(f, "["),
            Self::Brace => write!(f, "{{"),
        }
    }
}

impl Display for Punct {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::Ampersand => "&",
            Self::And => "&&",
            Self::PlusEq => "+=",
            Self::MinusEq => "-=",
            Self::StarEq => "*=",
            Self::SlashEq => "/=",
            Self::PercentEq => "%=",
            Self::Bang => "!",
            Self::Eq => "=",
            Self::EqEq => "==",
            Self::NotEq => "!=",
            Self::Less => "<",
            Self::LessEq => "<=",
            Self::Greater => ">",
            Self::GreaterEq => ">=",
            Self::Arrow => "->",
            Self::FatArrow => "=>",
            Self::Dot => ".",
            Self::Semicolon => ";",
            Self::Comma => ",",
            Self::Question => "?",
            Self::Colon => ":",
            Self::ColonColon => "::",
            Self::Underscore => "_",
            Self::Pipe => "|",
            Self::Or => "||",
        };
        write!(f, "{s}")
    }
}

impl Display for Kw {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Self::Let => "let",
            Self::Fn => "fn",
            Self::Return => "return",
            Self::If => "if",
            Self::Else => "else",
            Self::While => "while",
            Self::Loop => "loop",
            Self::For => "for",
            Self::In => "in",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Mod => "mod",
            Self::Match => "match",
            Self::Impl => "impl",
            Self::Pub => "pub",
            Self::Mut => "mut",
            Self::Type => "type",
            Self::As => "as",
            Self::True => "true",
            Self::False => "false",
            Self::Use => "use",
            Self::Where => "where",
            Self::Extern => "extern",
            Self::Const => "const",
        };
        write!(f, "{s}")
    }
}
