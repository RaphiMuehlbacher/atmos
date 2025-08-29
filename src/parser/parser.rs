use crate::error::CompilerError;
use crate::extension::SourceSpanExt;
use crate::lexer::token_kind::{Delimiter, Keyword, Literal, Punct};
use crate::lexer::{Token, TokenKind};
use crate::parser::ast::{
    AstNode, Crate, Expr, FnDecl, FnSig, GenericArg, GenericParam, GenericParamKind, Ident, Item,
    Path, PathSegment, Stmt, Ty,
};
use crate::parser::ParserError;
use crate::parser::ParserError::UnexpectedClosingDelimiter;
use crate::Session;

type PResult<T> = Result<T, ParserError>;

pub struct Parser<'a> {
    session: &'a Session,
    tokens: Vec<Token>,
    position: usize,
    delimiter_stack: Vec<Token>,
}

impl<'a> Parser<'a> {
    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn next(&self) -> &Token {
        &self.tokens[self.position + 1]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.position - 1]
    }

    fn at_eof(&self) -> bool {
        self.current().kind == TokenKind::EOF
    }

    fn advance(&mut self) {
        if !self.at_eof() {
            self.position += 1;
        }
    }

    fn current_is(&self, kind: TokenKind) -> bool {
        match (&self.current().kind, &kind) {
            (TokenKind::Literal(Literal::I32(_)), TokenKind::Literal(Literal::I32(_))) => true,
            (TokenKind::Literal(Literal::U32(_)), TokenKind::Literal(Literal::U32(_))) => true,
            (TokenKind::Literal(Literal::F64(_)), TokenKind::Literal(Literal::F64(_))) => true,
            (TokenKind::Literal(Literal::Str(_)), TokenKind::Literal(Literal::Str(_))) => true,
            (TokenKind::Ident(_), TokenKind::Ident(_)) => true,
            (a, b) => a == b,
        }
    }

    /// token to check is `current`
    fn check(&self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.current_is(kind.clone()) {
                return true;
            }
        }
        false
    }

    /// token to consume is `current`
    fn consume(&mut self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.current_is(kind.clone()) {
                self.advance();
                return true;
            }
        }
        false
    }
}

impl<'a> Parser<'a> {
    fn open_delimiter(&mut self, open_delimiter: TokenKind) {
        let token = self.current().clone();

        if token.kind != open_delimiter {
            self.emit(ParserError::UnexpectedToken {
                src: self.session.get_named_source(),
                span: token.span,
                found: token.kind,
                expected: open_delimiter,
            });
            self.advance();
            return;
        }

        match open_delimiter {
            TokenKind::OpeningDelimiter(_) => {
                self.delimiter_stack
                    .push(Token::new(open_delimiter, token.span));
                self.advance();
            }
            found => {
                self.emit(ParserError::UnexpectedToken {
                    src: self.session.get_named_source(),
                    span: token.span,
                    found,
                    expected: TokenKind::EOF,
                });
                self.advance();
            }
        }
    }

    fn close_delimiter(&mut self, close_delimiter: TokenKind) {
        let token = self.current().clone();

        if token.kind != close_delimiter {
            self.emit(ParserError::UnexpectedToken {
                src: self.session.get_named_source(),
                span: token.span,
                found: token.kind,
                expected: close_delimiter,
            });
            self.advance();
            return;
        }

        if self.delimiter_stack.is_empty() {
            self.emit(UnexpectedClosingDelimiter {
                src: self.session.get_named_source(),
                span: token.span,
                found_delimiter: token.kind,
            });
            self.advance();
            return;
        }

        let open_delim = self.delimiter_stack.pop().unwrap();
        let expected_closing = match open_delim.kind {
            TokenKind::OpeningDelimiter(Delimiter::Paren) => {
                TokenKind::ClosingDelimiter(Delimiter::Paren)
            }
            TokenKind::OpeningDelimiter(Delimiter::Bracket) => {
                TokenKind::ClosingDelimiter(Delimiter::Bracket)
            }
            TokenKind::OpeningDelimiter(Delimiter::Brace) => {
                TokenKind::ClosingDelimiter(Delimiter::Brace)
            }
            _ => unreachable!(),
        };

        if close_delimiter != expected_closing {
            self.emit(ParserError::MismatchedDelimiter {
                src: self.session.get_named_source(),
                closing_span: token.span,
                opening_span: open_delim.span,
                found: token.kind,
                expected: close_delimiter,
            })
        }
    }
    fn emit(&mut self, error: ParserError) {
        self.session.push_error(CompilerError::ParserError(error))
    }

    fn recover_item(&mut self) -> AstNode<Item> {
        while !self.at_eof() && !self.token_begins_item() {
            self.advance();
        }
        AstNode::err(Item::Err)
    }

    fn recover_stmt(&mut self) -> AstNode<Stmt> {
        while !self.at_eof() && !self.token_ends_stmt() {
            self.advance();
        }
        AstNode::err(Stmt::Err)
    }

    fn token_ends_stmt(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Punctuation(Punct::Semicolon)
                | TokenKind::ClosingDelimiter(Delimiter::Brace)
        )
    }

    fn token_begins_item(&self) -> bool {
        matches!(
            self.current().kind,
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

impl<'a> Parser<'a> {
    pub fn new(session: &'a Session, tokens: Vec<Token>) -> Self {
        Self {
            session,
            tokens,
            position: 0,
            delimiter_stack: vec![],
        }
    }

    pub fn parse_crate(&mut self) -> Crate {
        let lo = self.current().span;
        let mut items = vec![];

        while !self.at_eof() {
            items.push(self.parse_item());
        }

        Crate {
            items,
            span: lo.to(self.previous().span),
        }
    }

    fn parse_item(&mut self) -> AstNode<Item> {
        match self.parse_item_without_recovery() {
            Ok(item) => item,
            Err(err) => {
                self.session.push_error(CompilerError::ParserError(err));
                self.recover_item()
            }
        }
    }

    fn parse_item_without_recovery(&mut self) -> PResult<AstNode<Item>> {
        if self.check(&[TokenKind::Keyword(Keyword::Fn)]) {
            self.parse_fn_item()
        } else {
            todo!()
        }
    }

    /// fn a(b: i32) -> i32 { b }
    /// starts at 'fn', ends after the block
    fn parse_fn_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let sig = self.parse_fn_sig()?;

        Ok(AstNode::new(
            Item::Fn(FnDecl { sig, body: todo!() }),
            lo.to(self.previous().span),
        ))
    }

    // starts at the function name, ends before the block
    fn parse_fn_sig(&mut self) -> PResult<AstNode<FnSig>> {
        let lo = self.current().span;

        let ident = self.parse_ident()?;
        let generics = self.parse_generic_params()?;

        self.open_delimiter(TokenKind::OpeningDelimiter(Delimiter::Paren));
        self.close_delimiter(TokenKind::ClosingDelimiter(Delimiter::Paren));

        dbg!(&self.delimiter_stack);
        self.session.error_handler.borrow().emit_all();

        Ok(AstNode::new(
            FnSig {
                ident,
                generics,
                params: todo!(),
                return_ty: todo!(),
            },
            lo.to(self.previous().span),
        ))
    }

    fn parse_generic_params(&mut self) -> PResult<Vec<AstNode<GenericParam>>> {
        let mut generics = vec![];

        if !self.consume(&[TokenKind::Punctuation(Punct::Less)]) {
            return Ok(generics);
        }

        if self.consume(&[TokenKind::Punctuation(Punct::Greater)]) {
            return Ok(generics);
        }

        loop {
            let lo = self.current().span;

            let is_const = self.consume(&[TokenKind::Keyword(Keyword::Const)]);
            let ident = self.parse_ident()?;

            let bounds = if !is_const && self.consume(&[TokenKind::Punctuation(Punct::Colon)]) {
                self.parse_bounds()?
            } else {
                vec![]
            };

            let kind = if is_const {
                self.consume(&[TokenKind::Punctuation(Punct::Colon)]);
                let const_ty = self.parse_type()?;
                GenericParamKind::Const(const_ty)
            } else {
                GenericParamKind::Type
            };

            generics.push(AstNode::new(
                GenericParam {
                    ident,
                    bounds,
                    kind,
                },
                lo.to(self.previous().span),
            ));

            if self.consume(&[TokenKind::Punctuation(Punct::Greater)]) {
                break;
            }

            if !self.consume(&[TokenKind::Punctuation(Punct::Comma)]) {
                panic!()
            }
        }
        Ok(generics)
    }

    fn parse_bounds(&mut self) -> PResult<Vec<AstNode<Path>>> {
        let mut bounds = vec![];
        loop {
            let path = self.parse_path()?;
            bounds.push(path);
            if !self.consume(&[TokenKind::Punctuation(Punct::Plus)]) {
                break;
            }
        }
        Ok(bounds)
    }

    fn parse_path(&mut self) -> PResult<AstNode<Path>> {
        let lo = self.current().span;
        let mut segments = vec![];

        segments.push(self.parse_path_segment()?);

        while self.consume(&[TokenKind::Punctuation(Punct::ColonColon)]) {
            segments.push(self.parse_path_segment()?);
        }
        Ok(AstNode::new(Path { segments }, lo.to(self.previous().span)))
    }

    fn parse_path_segment(&mut self) -> PResult<AstNode<PathSegment>> {
        let lo = self.current().span;
        let ident = self.parse_ident()?;
        let args = self.parse_generic_args()?;

        Ok(AstNode::new(
            PathSegment { ident, args },
            lo.to(self.current().span),
        ))
    }

    fn parse_generic_args(&mut self) -> PResult<Vec<AstNode<GenericArg>>> {
        let mut args = vec![];
        if !self.consume(&[TokenKind::Punctuation(Punct::Less)]) {
            return Ok(args);
        }
        if self.consume(&[TokenKind::Punctuation(Punct::Greater)]) {
            panic!()
        }

        loop {
            if self.consume(&[TokenKind::Punctuation(Punct::Greater)]) {
                break;
            }

            let lo = self.current().span;

            let generic_arg = if self.consume(&[TokenKind::Keyword(Keyword::Const)]) {
                let expr = self.parse_expression()?;
                GenericArg::Const(Box::new(expr))
            } else {
                let ty = self.parse_type()?;
                GenericArg::Type(ty)
            };
            args.push(AstNode::new(generic_arg, lo.to(self.current().span)));

            if self.consume(&[TokenKind::Punctuation(Punct::Greater)]) {
                break;
            }

            if !self.consume(&[TokenKind::Punctuation(Punct::Comma)]) {
                panic!()
            }
        }

        Ok(args)
    }

    fn parse_type(&mut self) -> PResult<AstNode<Ty>> {
        let lo = self.current().span;
        let ty = match &self.current().kind {
            TokenKind::Keyword(Keyword::Fn) => {
                let sig = self.parse_fn_sig()?;
                Ty::Fn(Box::new(sig))
            }
            TokenKind::OpeningDelimiter(Delimiter::Bracket) => {
                self.advance();
                let inner = self.parse_type()?;
                if !self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]) {
                    panic!()
                }
                let len = self.parse_expression()?;

                if !self.consume(&[TokenKind::ClosingDelimiter(Delimiter::Bracket)]) {
                    panic!()
                }
                Ty::Array(Box::new(inner), Box::new(len))
            }
            TokenKind::Punctuation(Punct::Star) => {
                self.advance();
                let ty = self.parse_type()?;
                Ty::Ptr(Box::new(ty))
            }
            TokenKind::OpeningDelimiter(Delimiter::Paren) => {
                self.advance();
                if self.consume(&[TokenKind::ClosingDelimiter(Delimiter::Paren)]) {
                    Ty::Tuple(vec![])
                } else {
                    let mut elems = vec![];
                    elems.push(self.parse_type()?);
                    while self.consume(&[TokenKind::Punctuation(Punct::Comma)]) {
                        if self.consume(&[TokenKind::ClosingDelimiter(Delimiter::Paren)]) {
                            return Ok(AstNode::new(Ty::Tuple(elems), lo.to(self.previous().span)));
                        }
                        elems.push(self.parse_type()?);
                    }

                    if !self.consume(&[TokenKind::ClosingDelimiter(Delimiter::Paren)]) {
                        panic!()
                    }
                    Ty::Tuple(elems)
                }
            }

            _ => Ty::Path(self.parse_path()?),
        };
        Ok(AstNode::new(ty, lo.to(self.previous().span)))
    }

    fn parse_ident(&mut self) -> PResult<AstNode<Ident>> {
        let token = self.current().clone();
        let lo = token.span;

        match &token.kind {
            TokenKind::Ident(ident) => {
                self.advance();
                Ok(AstNode::new(
                    Ident::new(ident.into()),
                    lo.to(self.previous().span),
                ))
            }
            found => {
                self.emit(ParserError::ExpectedIdentifier {
                    src: self.session.get_named_source(),
                    found: found.clone(),
                    span: token.span,
                });
                Ok(AstNode::err(Ident::err()))
            }
        }
    }

    fn parse_expression(&mut self) -> PResult<AstNode<Expr>> {
        todo!()
    }
}
