use crate::lexer::token_kind::{Delimiter, Kw, Literal, Punct};
use crate::lexer::{LexerError, Token, TokenKind};
use crate::Session;
use miette::SourceSpan;

pub struct Lexer<'sess> {
    session: &'sess Session,
    position: usize,
}

impl<'sess> Lexer<'sess> {
    pub fn new(session: &'sess Session) -> Self {
        Self { session, position: 0 }
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
                                Some('*') if self.match_char('/') => depth -= 1,
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
                '&' => {
                    if self.match_char('&') {
                        TokenKind::Punctuation(Punct::And)
                    } else {
                        TokenKind::Punctuation(Punct::Ampersand)
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
                '_' if self.is_standalone_underscore() => TokenKind::Punctuation(Punct::Underscore),
                '|' => {
                    if self.match_char('|') {
                        TokenKind::Punctuation(Punct::Or)
                    } else {
                        TokenKind::Punctuation(Punct::Pipe)
                    }
                }
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

            let token = Token::new(kind, SourceSpan::new(start.into(), self.position - start));
            tokens.push(token);
        }

        tokens.push(Token::new(TokenKind::EOF, SourceSpan::from((self.position, 0))));
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
        let mut value = String::new();
        let mut has_dot = false;

        value.push(self.session.get_source()[self.position - 1..].chars().next().unwrap());

        while let Some(c) = self.peek() {
            match c {
                '_' => {
                    self.advance();
                }
                '0'..='9' => {
                    value.push(c);
                    self.advance();
                }
                '.' if !has_dot && self.peek_next().is_some_and(|c| c.is_ascii_digit()) => {
                    has_dot = true;
                    value.push(c);
                    self.advance();
                }
                _ => break,
            }
        }

        let suffix = self.peek().is_some_and(|c| self.is_ident_start(c)).then(|| {
            self.advance();
            self.collect_identifier_string()
        });

        let literal = if has_dot {
            Literal::Float { value, suffix }
        } else {
            Literal::Integer { value, suffix }
        };

        TokenKind::Literal(literal)
    }

    fn collect_identifier_string(&mut self) -> String {
        let mut value = String::new();
        value.push(self.session.get_source()[self.position - 1..].chars().next().unwrap());

        while let Some(c) = self.peek() {
            if self.is_ident_continue(c) {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        value
    }

    fn lex_identifier(&mut self) -> TokenKind {
        let value = self.collect_identifier_string();

        match value.as_str() {
            "let" => TokenKind::Keyword(Kw::Let),
            "fn" => TokenKind::Keyword(Kw::Fn),
            "return" => TokenKind::Keyword(Kw::Return),
            "if" => TokenKind::Keyword(Kw::If),
            "else" => TokenKind::Keyword(Kw::Else),
            "while" => TokenKind::Keyword(Kw::While),
            "loop" => TokenKind::Keyword(Kw::Loop),
            "for" => TokenKind::Keyword(Kw::For),
            "in" => TokenKind::Keyword(Kw::In),
            "break" => TokenKind::Keyword(Kw::Break),
            "continue" => TokenKind::Keyword(Kw::Continue),
            "struct" => TokenKind::Keyword(Kw::Struct),
            "enum" => TokenKind::Keyword(Kw::Enum),
            "trait" => TokenKind::Keyword(Kw::Trait),
            "mod" => TokenKind::Keyword(Kw::Mod),
            "match" => TokenKind::Keyword(Kw::Match),
            "impl" => TokenKind::Keyword(Kw::Impl),
            "pub" => TokenKind::Keyword(Kw::Pub),
            "mut" => TokenKind::Keyword(Kw::Mut),
            "type" => TokenKind::Keyword(Kw::Type),
            "as" => TokenKind::Keyword(Kw::As),
            "true" => TokenKind::Keyword(Kw::True),
            "false" => TokenKind::Keyword(Kw::False),
            "use" => TokenKind::Keyword(Kw::Use),
            "where" => TokenKind::Keyword(Kw::Where),
            "extern" => TokenKind::Keyword(Kw::Extern),
            "const" => TokenKind::Keyword(Kw::Const),
            "unit" => TokenKind::Keyword(Kw::Unit),
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

    fn is_standalone_underscore(&self) -> bool {
        !matches!(self.peek(), Some(next) if self.is_ident_continue(next))
    }

    fn peek(&self) -> Option<char> {
        self.session.get_source()[self.position..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let src = self.session.get_source();
        if self.position + 1 <= src.len() {
            src[self.position + 1..].chars().next()
        } else {
            None
        }
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
