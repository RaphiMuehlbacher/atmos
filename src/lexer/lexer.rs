use crate::Session;
use crate::lexer::token_kind::{Delimiter, Keyword, Literal, Punct};
use crate::lexer::{LexerError, Token, TokenKind};
use miette::SourceSpan;

pub struct Lexer<'sess> {
    session: &'sess Session,
    position: usize,
}

impl<'sess> Lexer<'sess> {
    pub fn new(session: &'sess Session) -> Self {
        Self {
            session,
            position: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = vec![];

        while let Some(c) = self.advance() {
            let start = self.position - c.len_utf8();

            let kind = match c {
                '(' => TokenKind::OpeningDelimiter(Delimiter::Paren),
                ')' => TokenKind::ClosingDelimiter(Delimiter::Paren),
                '[' => TokenKind::OpeningDelimiter(Delimiter::Bracket),
                ']' => TokenKind::ClosingDelimiter(Delimiter::Bracket),
                '{' => TokenKind::OpeningDelimiter(Delimiter::Brace),
                '}' => TokenKind::ClosingDelimiter(Delimiter::Brace),
                '+' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::PlusEq)
                    } else {
                        TokenKind::Punctuation(Punct::Plus)
                    }
                }
                '-' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::MinusEq)
                    } else if self.match_char('>') {
                        TokenKind::Punctuation(Punct::Arrow)
                    } else {
                        TokenKind::Punctuation(Punct::Minus)
                    }
                }
                '*' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::StarEq)
                    } else {
                        TokenKind::Punctuation(Punct::Star)
                    }
                }
                '/' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::SlashEq)
                    } else if self.match_char('/') {
                        while let Some(c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.advance();
                        }
                        continue;
                    } else if self.match_char('*') {
                        let mut depth = 1;
                        while depth > 0 {
                            match self.advance() {
                                Some('/') if self.match_char('*') => depth += 1,
                                Some('*') if self.match_char('/') => depth += 1,
                                None => {
                                    self.session.push_error(
                                        LexerError::UnterminatedComment {
                                            src: self.session.get_named_source(),
                                            span: (start, 1).into(),
                                        }
                                        .into(),
                                    );
                                    break;
                                }
                                _ => {}
                            }
                        }
                        continue;
                    } else {
                        TokenKind::Punctuation(Punct::Slash)
                    }
                }
                '%' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::PercentEq)
                    } else {
                        TokenKind::Punctuation(Punct::Percent)
                    }
                }
                '=' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::EqEq)
                    } else if self.match_char('>') {
                        TokenKind::Punctuation(Punct::FatArrow)
                    } else {
                        TokenKind::Punctuation(Punct::Eq)
                    }
                }
                '!' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::NotEq)
                    } else {
                        TokenKind::Punctuation(Punct::Bang)
                    }
                }
                '<' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::LessEq)
                    } else {
                        TokenKind::Punctuation(Punct::Less)
                    }
                }
                '>' => {
                    if self.match_char('=') {
                        TokenKind::Punctuation(Punct::GreaterEq)
                    } else {
                        TokenKind::Punctuation(Punct::Greater)
                    }
                }
                '.' => TokenKind::Punctuation(Punct::Dot),
                ',' => TokenKind::Punctuation(Punct::Comma),
                ';' => TokenKind::Punctuation(Punct::Semicolon),
                '?' => TokenKind::Punctuation(Punct::Question),
                '_' => TokenKind::Punctuation(Punct::Underscore),
                '|' => TokenKind::Punctuation(Punct::Pipe),
                ':' => {
                    if self.match_char(':') {
                        TokenKind::Punctuation(Punct::ColonColon)
                    } else {
                        TokenKind::Punctuation(Punct::Colon)
                    }
                }
                '"' => self.lex_string(),
                c if c.is_ascii_digit() => self.lex_number(),
                c if c.is_whitespace() => {
                    self.skip_whitespace();
                    continue;
                }
                c if self.is_ident_start(c) => self.lex_identifier(),
                c => {
                    self.session.push_error(
                        LexerError::UnexpectedCharacter {
                            src: self.session.get_named_source(),
                            character: c,
                            span: start.into(),
                        }
                        .into(),
                    );
                    continue;
                }
            };

            let token = Token::new(kind, SourceSpan::from((start, self.position - start)));
            tokens.push(token);
        }

        tokens.push(Token::new(
            TokenKind::EOF,
            SourceSpan::from((self.position - 1, 1)),
        ));
        tokens
    }

    fn lex_string(&mut self) -> TokenKind {
        let mut value = String::new();
        let start_quote = self.position;
        while let Some(c) = self.peek() {
            self.advance();
            match c {
                '"' => return TokenKind::Literal(Literal::Str(value)),
                '\\' => {
                    if let Some(next) = self.advance() {
                        match next {
                            'n' => value.push('\n'),
                            't' => value.push('\t'),
                            'r' => value.push('\r'),
                            '\\' => value.push('\\'),
                            '"' => value.push('"'),
                            _ => self.session.push_error(
                                LexerError::InvalidEscapeCharacter {
                                    src: self.session.get_named_source(),
                                    span: (self.position - 1).into(),
                                    character: next,
                                }
                                .into(),
                            ),
                        }
                    }
                }
                _ => value.push(c),
            }
        }
        self.session.push_error(
            LexerError::UnterminatedString {
                src: self.session.get_named_source(),
                span: (start_quote - 1).into(),
            }
            .into(),
        );
        TokenKind::Literal(Literal::Str(value))
    }

    fn lex_number(&mut self) -> TokenKind {
        let mut value = String::from(
            self.session
                .get_source()
                .chars()
                .nth(self.position - 1)
                .unwrap(),
        );
        let mut has_dot = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                value.push(c);
                self.advance();
            } else if c == '.' && !has_dot {
                has_dot = true;
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if self.check_suffix("u32") {
            TokenKind::Literal(Literal::U32(value.parse().unwrap()))
        } else if has_dot {
            TokenKind::Literal(Literal::F64(value.parse().unwrap()))
        } else {
            TokenKind::Literal(Literal::I32(value.parse().unwrap()))
        }
    }

    fn check_suffix(&mut self, suffix: &str) -> bool {
        let remaining = &self.session.get_source()[self.position..];
        if remaining.starts_with(suffix) {
            self.position += suffix.len();
            true
        } else {
            false
        }
    }

    fn lex_identifier(&mut self) -> TokenKind {
        let mut value = String::new();
        value.push(
            self.session
                .get_source()
                .chars()
                .nth(self.position - 1)
                .unwrap(),
        );

        while let Some(c) = self.peek() {
            if self.is_ident_continue(c) {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        match value.as_str() {
            "let" => TokenKind::Keyword(Keyword::Let),
            "fn" => TokenKind::Keyword(Keyword::Fn),
            "return" => TokenKind::Keyword(Keyword::Return),
            "if" => TokenKind::Keyword(Keyword::If),
            "else" => TokenKind::Keyword(Keyword::Else),
            "while" => TokenKind::Keyword(Keyword::While),
            "for" => TokenKind::Keyword(Keyword::For),
            "in" => TokenKind::Keyword(Keyword::In),
            "break" => TokenKind::Keyword(Keyword::Break),
            "continue" => TokenKind::Keyword(Keyword::Continue),
            "struct" => TokenKind::Keyword(Keyword::Struct),
            "enum" => TokenKind::Keyword(Keyword::Enum),
            "trait" => TokenKind::Keyword(Keyword::Trait),
            "match" => TokenKind::Keyword(Keyword::Match),
            "impl" => TokenKind::Keyword(Keyword::Impl),
            "pub" => TokenKind::Keyword(Keyword::Pub),
            "mut" => TokenKind::Keyword(Keyword::Mut),
            "type" => TokenKind::Keyword(Keyword::Type),
            "as" => TokenKind::Keyword(Keyword::As),
            "true" => TokenKind::Keyword(Keyword::True),
            "false" => TokenKind::Keyword(Keyword::False),
            "null" => TokenKind::Keyword(Keyword::Null),
            "self" => TokenKind::Keyword(Keyword::SelfKw),
            "super" => TokenKind::Keyword(Keyword::Super),
            "use" => TokenKind::Keyword(Keyword::Use),
            "where" => TokenKind::Keyword(Keyword::Where),
            "extern" => TokenKind::Keyword(Keyword::Extern),
            "const" => TokenKind::Keyword(Keyword::Const),
            _ => TokenKind::Ident(value),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if let Some(c) = self.peek()
            && c == expected
        {
            self.advance();
            return true;
        }
        false
    }

    fn is_ident_start(&self, c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    fn is_ident_continue(&self, c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    fn peek(&self) -> Option<char> {
        self.session.get_source()[self.position..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(c) = self.peek() {
            self.position += c.len_utf8();
            Some(c)
        } else {
            None
        }
    }
}
