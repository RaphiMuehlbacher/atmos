use crate::error::CompilerError;
use crate::extension::SourceSpanExt;
use crate::lexer::token_kind::{Delimiter, Keyword, Literal, Punct};
use crate::lexer::{Token, TokenKind};
use crate::parser::ast::{
    ArrayExpr, AstNode, BlockExpr, CallExpr, Crate, Expr, FieldAccessExpr, FnDecl, FnSig,
    GenericArg, GenericParam, GenericParamKind, Ident, IfExpr, IndexExpr, Item, LetStmt,
    LiteralExpr, Param, Path, PathExpr, PathSegment, Pattern, Stmt, TupleExpr, Ty, UnOp, UnaryExpr,
    WhileExpr,
};
use crate::parser::ParserError;
use crate::Session;

type PResult<T> = Result<T, ParserError>;

pub struct Parser<'a> {
    session: &'a Session,
    tokens: Vec<Token>,
    position: usize,
}

impl<'a> Parser<'a> {
    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.position - 1]
    }

    fn at_eof(&self) -> bool {
        self.current_is(&TokenKind::EOF)
    }

    fn advance(&mut self) {
        if !self.at_eof() {
            self.position += 1;
        }
    }

    fn current_is(&self, kind: &TokenKind) -> bool {
        match (&self.current().kind, kind) {
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
            if self.current_is(kind) {
                return true;
            }
        }
        false
    }

    /// token to consume is `current`
    fn consume(&mut self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.current_is(kind) {
                self.advance();
                return true;
            }
        }
        false
    }
}

impl<'a> Parser<'a> {
    fn parse_separated_delimited<T, F>(
        &mut self,
        open: TokenKind,
        close: TokenKind,
        separator: TokenKind,
        parse_element: F,
    ) -> Vec<T>
    where
        F: FnMut(&mut Self) -> PResult<T>,
    {
        self.parse_separated_delimited_with_trailing(open, close, separator, parse_element)
            .0
    }

    fn parse_separated_delimited_with_trailing<T, F>(
        &mut self,
        open: TokenKind,
        close: TokenKind,
        separator: TokenKind,
        mut parse_element: F,
    ) -> (Vec<T>, bool)
    where
        F: FnMut(&mut Self) -> PResult<T>,
    {
        let open_span = self.current().span;
        let mut delimiter_err_emitted = false;

        if self.current_is(&open) {
            self.advance();
        } else {
            self.emit(ParserError::UnexpectedToken {
                src: self.session.get_named_source(),
                span: self.current().span,
                found: self.current().kind.clone(),
                expected: open,
            });

            delimiter_err_emitted = true;

            if self.is_junk_for_delim(&self.current().kind.clone()) {
                self.advance();
            }
        }

        let mut elements = vec![];
        let mut trailing_comma = false;
        loop {
            if self.current_is(&close) || self.at_eof() {
                break;
            }

            match parse_element(self) {
                Ok(element) => elements.push(element),
                Err(err) => {
                    self.emit(err);
                    self.recover_to_separator_or_closing(&close, &separator);
                }
            }

            if self.current_is(&separator) {
                self.advance();
                trailing_comma = true;
                if self.current_is(&close) {
                    break;
                }
            } else if !self.current_is(&close) {
                self.recover_to_separator_or_closing(&close, &separator);
                trailing_comma = false;
            } else {
                trailing_comma = false;
            }
        }

        if self.current_is(&close) {
            self.advance();
        } else if !delimiter_err_emitted {
            match &self.current().kind {
                TokenKind::EOF => {
                    self.emit(ParserError::UnclosedDelimiter {
                        src: self.session.get_named_source(),
                        span: self.current().span,
                        delimiter: close,
                    });
                }
                other_delimiter if matches!(other_delimiter, TokenKind::ClosingDelimiter(_)) => {
                    self.emit(ParserError::MismatchedDelimiter {
                        src: self.session.get_named_source(),
                        closing_span: self.current().span,
                        opening_span: open_span,
                        found: other_delimiter.clone(),
                        expected: close,
                    });
                    self.advance();
                }
                other => {
                    self.emit(ParserError::UnexpectedToken {
                        src: self.session.get_named_source(),
                        span: self.current().span,
                        found: other.clone(),
                        expected: close,
                    });
                }
            }
        }
        (elements, trailing_comma)
    }

    fn recover_to_separator_or_closing(&mut self, close: &TokenKind, separator: &TokenKind) {
        while !self.at_eof() && !self.current_is(close) && !self.current_is(separator) {
            self.advance();
        }
    }

    fn is_junk_for_delim(&self, token: &TokenKind) -> bool {
        match token {
            TokenKind::ClosingDelimiter(_) | TokenKind::OpeningDelimiter(_) => false,
            TokenKind::Ident(_) | TokenKind::Literal(_) => false,
            TokenKind::Keyword(_) => true,
            TokenKind::Punctuation(_) => true,
            _ => false,
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
        }
    }

    pub fn parse_crate(&mut self) -> Crate {
        let lo = self.current().span;
        let mut items = vec![];

        while !self.at_eof() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(err) => {
                    self.emit(err);
                    self.recover_item();
                }
            }
        }

        Crate {
            items,
            span: lo.to(self.previous().span),
        }
    }

    fn parse_item(&mut self) -> PResult<AstNode<Item>> {
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
        let body = self.parse_block()?;

        Ok(AstNode::new(
            Item::Fn(FnDecl { sig, body }),
            lo.to(self.previous().span),
        ))
    }

    // starts at the function name, ends before the block
    fn parse_fn_sig(&mut self) -> PResult<AstNode<FnSig>> {
        let lo = self.current().span;

        let ident = self.parse_ident()?;
        let generics = self.parse_generic_params()?;
        let params = self.parse_params()?;
        let return_ty = self.parse_return_type()?;

        Ok(AstNode::new(
            FnSig {
                ident,
                generics,
                params,
                return_ty,
            },
            lo.to(self.previous().span),
        ))
    }

    /// starts at '->', ends after the type
    fn parse_return_type(&mut self) -> PResult<Option<AstNode<Ty>>> {
        if self.consume(&[TokenKind::Punctuation(Punct::Arrow)]) {
            let ty = self.parse_type()?;
            Ok(Some(ty))
        } else {
            Ok(None)
        }
    }

    /// starts at '(', ends after ')'
    fn parse_params(&mut self) -> PResult<Vec<AstNode<Param>>> {
        Ok(self.parse_separated_delimited(
            TokenKind::OpeningDelimiter(Delimiter::Paren),
            TokenKind::ClosingDelimiter(Delimiter::Paren),
            TokenKind::Punctuation(Punct::Comma),
            |p| p.parse_param(),
        ))
    }

    /// starts at the identifier and ends after the type
    fn parse_param(&mut self) -> PResult<AstNode<Param>> {
        let lo = self.current().span;

        let pattern = self.parse_pattern()?;
        if !self.consume(&[TokenKind::Punctuation(Punct::Colon)]) {
            self.emit(ParserError::UnexpectedToken {
                src: self.session.get_named_source(),
                span: self.current().span,
                found: self.current().kind.clone(),
                expected: TokenKind::Punctuation(Punct::Colon),
            });
            if !matches!(self.current().kind, TokenKind::Ident(_)) {
                self.advance();
            }
        }
        let ty = self.parse_type()?;
        Ok(AstNode::new(
            Param {
                pattern,
                type_annotation: ty,
            },
            lo.to(self.previous().span),
        ))
    }

    fn parse_pattern(&mut self) -> PResult<AstNode<Pattern>> {
        let lo = self.current().span;

        let pattern = match &self.current().kind {
            TokenKind::Punctuation(Punct::Underscore) => {
                self.advance();
                Pattern::Wildcard
            }
            TokenKind::Ident(_) => {
                let ident = self.parse_ident()?;
                Pattern::Ident(ident)
            }
            TokenKind::OpeningDelimiter(Delimiter::Paren) => {
                let (elements, trailing_comma) = self.parse_separated_delimited_with_trailing(
                    TokenKind::OpeningDelimiter(Delimiter::Paren),
                    TokenKind::ClosingDelimiter(Delimiter::Paren),
                    TokenKind::Punctuation(Punct::Comma),
                    |p| p.parse_pattern(),
                );
                if elements.len() == 1 && !trailing_comma {
                    Pattern::Paren(Box::new(elements[0].clone()))
                } else {
                    Pattern::Tuple(elements)
                }
            }
            _ => panic!("Expected Pattern"),
        };

        Ok(AstNode::new(pattern, lo.to(self.previous().span)))
    }

    fn parse_generic_params(&mut self) -> PResult<Vec<AstNode<GenericParam>>> {
        if !self.current_is(&TokenKind::Punctuation(Punct::Less)) {
            return Ok(vec![]);
        }

        Ok(self.parse_separated_delimited(
            TokenKind::Punctuation(Punct::Less),
            TokenKind::Punctuation(Punct::Greater),
            TokenKind::Punctuation(Punct::Comma),
            |p| p.parse_generic_param(),
        ))
    }

    fn parse_generic_param(&mut self) -> PResult<AstNode<GenericParam>> {
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

        Ok(AstNode::new(
            GenericParam {
                ident,
                bounds,
                kind,
            },
            lo.to(self.previous().span),
        ))
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
            lo.to(self.previous().span),
        ))
    }
    fn parse_generic_args(&mut self) -> PResult<Vec<AstNode<GenericArg>>> {
        if !self.check(&[TokenKind::Punctuation(Punct::Less)]) {
            return Ok(vec![]);
        }

        Ok(self.parse_separated_delimited(
            TokenKind::Punctuation(Punct::Less),
            TokenKind::Punctuation(Punct::Greater),
            TokenKind::Punctuation(Punct::Comma),
            |p| p.parse_generic_arg(),
        ))
    }

    fn parse_generic_arg(&mut self) -> PResult<AstNode<GenericArg>> {
        let lo = self.current().span;

        let generic_arg = if self.consume(&[TokenKind::Keyword(Keyword::Const)]) {
            let expr = self.parse_expression()?;
            GenericArg::Const(Box::new(expr))
        } else {
            let ty = self.parse_type()?;
            GenericArg::Type(ty)
        };
        Ok(AstNode::new(generic_arg, lo.to(self.previous().span)))
    }

    fn parse_type(&mut self) -> PResult<AstNode<Ty>> {
        let lo = self.current().span;

        let ty = match &self.current().kind {
            TokenKind::Keyword(Keyword::Fn) => {
                self.advance();
                let param_types = self.parse_separated_delimited(
                    TokenKind::OpeningDelimiter(Delimiter::Paren),
                    TokenKind::ClosingDelimiter(Delimiter::Paren),
                    TokenKind::Punctuation(Punct::Comma),
                    |p| p.parse_type(),
                );

                let return_ty = self.parse_return_type()?;

                Ty::Fn(param_types, Box::new(return_ty))
            }
            TokenKind::OpeningDelimiter(Delimiter::Bracket) => {
                self.advance();
                let inner_ty = self.parse_type()?;

                if !self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]) {
                    self.emit(ParserError::UnexpectedToken {
                        src: self.session.get_named_source(),
                        span: self.current().span,
                        found: self.current().kind.clone(),
                        expected: TokenKind::Punctuation(Punct::Semicolon),
                    });
                }
                let len = self.parse_expression()?;

                if !self.consume(&[TokenKind::ClosingDelimiter(Delimiter::Bracket)]) {
                    self.emit(ParserError::UnexpectedToken {
                        src: self.session.get_named_source(),
                        span: self.current().span,
                        found: self.current().kind.clone(),
                        expected: TokenKind::ClosingDelimiter(Delimiter::Bracket),
                    });
                }
                Ty::Array(Box::new(inner_ty), Box::new(len))
            }
            TokenKind::Punctuation(Punct::Star) => {
                self.advance();
                let ty = self.parse_type()?;
                Ty::Ptr(Box::new(ty))
            }
            TokenKind::OpeningDelimiter(Delimiter::Paren) => {
                let (elements, trailing_comma) = self.parse_separated_delimited_with_trailing(
                    TokenKind::OpeningDelimiter(Delimiter::Paren),
                    TokenKind::ClosingDelimiter(Delimiter::Paren),
                    TokenKind::Punctuation(Punct::Comma),
                    |p| p.parse_type(),
                );

                if elements.len() == 1 && !trailing_comma {
                    elements[0].node.clone()
                } else {
                    Ty::Tuple(elements)
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

    /// start at '{', end at '}'
    fn parse_block(&mut self) -> PResult<AstNode<BlockExpr>> {
        let lo = self.current().span;
        self.advance();

        let mut stmts = vec![];

        while !self.consume(&[TokenKind::ClosingDelimiter(Delimiter::Brace)]) {
            if self.at_eof() {
                break;
            }

            let stmt = self.parse_statement()?;
            stmts.push(stmt);
        }
        Ok(AstNode::new(
            BlockExpr { stmts },
            lo.to(self.previous().span),
        ))
    }

    fn parse_statement(&mut self) -> PResult<AstNode<Stmt>> {
        let lo = self.current().span;

        let stmt = match self.current().kind {
            TokenKind::Keyword(Keyword::Let) => self.parse_let_stmt()?,

            _ if self.token_begins_item() => {
                let item = self.parse_item()?;
                AstNode::new(Stmt::Item(item), lo.to(self.previous().span))
            }
            _ => {
                let expr = self.parse_expression()?;
                if self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]) {
                    AstNode::new(Stmt::Semi(expr), lo.to(self.previous().span))
                } else {
                    AstNode::new(Stmt::Expr(expr), lo.to(self.previous().span))
                }
            }
        };

        Ok(stmt)
    }

    fn parse_let_stmt(&mut self) -> PResult<AstNode<Stmt>> {
        let lo = self.current().span;
        self.advance();

        let pat = self.parse_pattern()?;

        let type_annotation = if self.consume(&[TokenKind::Punctuation(Punct::Colon)]) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let expr = if self.consume(&[TokenKind::Punctuation(Punct::Eq)]) {
            let expr = self.parse_expression()?;
            Some(Box::new(expr))
        } else {
            None
        };

        if !self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]) {
            self.emit(ParserError::UnexpectedToken {
                src: self.session.get_named_source(),
                span: self.current().span,
                found: self.current().kind.clone(),
                expected: TokenKind::Punctuation(Punct::Semicolon),
            });
        }
        Ok(AstNode::new(
            Stmt::Let(LetStmt {
                pat,
                expr,
                type_annotation,
            }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_expression(&mut self) -> PResult<AstNode<Expr>> {
        self.parse_expr_with_precedence(0)
    }

    fn parse_expr_with_precedence(&mut self, min_prec: u8) -> PResult<AstNode<Expr>> {
        let mut lhs = self.parse_prefix()?;

        loop {
            match self.current().kind {
                TokenKind::OpeningDelimiter(Delimiter::Paren) => {
                    let args = self.parse_separated_delimited(
                        TokenKind::OpeningDelimiter(Delimiter::Paren),
                        TokenKind::ClosingDelimiter(Delimiter::Paren),
                        TokenKind::Punctuation(Punct::Comma),
                        |p| p.parse_expression(),
                    );

                    lhs = AstNode::new(
                        Expr::Call(CallExpr {
                            arguments: args,
                            callee: Box::new(lhs.clone()),
                        }),
                        lhs.span.to(self.previous().span),
                    )
                }
                TokenKind::Punctuation(Punct::Dot) => {
                    self.advance();

                    let field = self.parse_ident()?;
                    lhs = AstNode::new(
                        Expr::FieldAccess(FieldAccessExpr {
                            target: Box::new(lhs.clone()),
                            field,
                        }),
                        lhs.span.to(self.previous().span),
                    )
                }
                TokenKind::OpeningDelimiter(Delimiter::Bracket) => {
                    self.advance();
                    let expr = self.parse_expression()?;

                    if !self.consume(&[TokenKind::ClosingDelimiter(Delimiter::Bracket)]) {
                        self.emit(ParserError::UnexpectedToken {
                            src: self.session.get_named_source(),
                            span: self.current().span,
                            found: self.current().kind.clone(),
                            expected: TokenKind::ClosingDelimiter(Delimiter::Bracket),
                        });
                    }

                    lhs = AstNode::new(
                        Expr::Index(IndexExpr {
                            target: Box::new(lhs.clone()),
                            index: Box::new(expr),
                        }),
                        lhs.span.to(self.previous().span),
                    )
                }
                _ => break,
            }
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> PResult<AstNode<Expr>> {
        let lo = self.current().span;

        let op = match self.current().kind {
            TokenKind::Punctuation(Punct::Bang) => UnOp::Not,
            TokenKind::Punctuation(Punct::Minus) => UnOp::Neg,
            TokenKind::Punctuation(Punct::Star) => UnOp::Deref,
            TokenKind::Punctuation(Punct::Ampersand) => UnOp::AddrOf,
            _ => {
                return self.parse_atom();
            }
        };

        let op = AstNode::new(op, self.current().span);
        self.advance();

        let operand = self.parse_prefix()?;

        Ok(AstNode::new(
            Expr::Unary(UnaryExpr {
                operator: op,
                operand: Box::new(operand),
            }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_atom(&mut self) -> PResult<AstNode<Expr>> {
        let lo = self.current().span;

        let atom = match &self.current().kind {
            TokenKind::Keyword(Keyword::True) => {
                self.advance();
                Expr::Literal(LiteralExpr::Bool(true))
            }
            TokenKind::Keyword(Keyword::False) => {
                self.advance();
                Expr::Literal(LiteralExpr::Bool(false))
            }
            TokenKind::Keyword(Keyword::Unit) => {
                self.advance();
                Expr::Literal(LiteralExpr::Unit)
            }
            TokenKind::Literal(lit) => {
                let expr = match lit {
                    Literal::I32(i32) => Expr::Literal(LiteralExpr::I32(*i32)),
                    Literal::U32(u32) => Expr::Literal(LiteralExpr::U32(*u32)),
                    Literal::F64(f64) => Expr::Literal(LiteralExpr::F64(*f64)),
                    Literal::Str(str) => Expr::Literal(LiteralExpr::Str(str.clone())),
                };
                self.advance();
                expr
            }
            TokenKind::Ident(_) => {
                let path = self.parse_path()?;
                Expr::Path(PathExpr { path })
            }
            TokenKind::OpeningDelimiter(Delimiter::Bracket) => {
                let elems = self.parse_separated_delimited(
                    TokenKind::OpeningDelimiter(Delimiter::Bracket),
                    TokenKind::ClosingDelimiter(Delimiter::Bracket),
                    TokenKind::Punctuation(Punct::Comma),
                    |p| p.parse_expression(),
                );
                Expr::Array(ArrayExpr { expressions: elems })
            }
            TokenKind::OpeningDelimiter(Delimiter::Paren) => {
                let (elems, trailing_comma) = self.parse_separated_delimited_with_trailing(
                    TokenKind::OpeningDelimiter(Delimiter::Paren),
                    TokenKind::ClosingDelimiter(Delimiter::Paren),
                    TokenKind::Punctuation(Punct::Comma),
                    |p| p.parse_expression(),
                );
                if elems.len() == 1 && !trailing_comma {
                    Expr::Paren(Box::new(elems[0].clone()))
                } else {
                    Expr::Tuple(TupleExpr { expressions: elems })
                }
            }
            TokenKind::OpeningDelimiter(Delimiter::Brace) => {
                let block = self.parse_block()?;
                Expr::Block(block.node)
            }
            TokenKind::Keyword(Keyword::If) => {
                let if_expr = self.parse_if_expr()?;
                Expr::If(if_expr.node)
            }

            TokenKind::Keyword(Keyword::While) => {
                let while_expr = self.parse_while_expr()?;
                Expr::While(while_expr.node)
            }

            _ => {
                panic!()
            }
        };

        Ok(AstNode::new(atom, lo.to(self.previous().span)))
    }

    fn parse_if_expr(&mut self) -> PResult<AstNode<IfExpr>> {
        let lo = self.current().span;

        self.advance();
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.consume(&[TokenKind::Keyword(Keyword::Else)]) {
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };

        Ok(AstNode::new(
            IfExpr {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch,
            },
            lo.to(self.previous().span),
        ))
    }

    fn parse_while_expr(&mut self) -> PResult<AstNode<WhileExpr>> {
        todo!()
    }
}
