use crate::error::CompilerError;
use crate::extension::SourceSpanExt;
use crate::lexer::token_kind::{Delimiter, Kw, Literal, Punct};
use crate::lexer::{Token, TokenKind};
use crate::parser::ast::{
    ArrayExpr, AssignExpr, AssignOp, AssignOpExpr, AssociatedItem, AstNode, BinOp, BinaryExpr, BlockExpr, BreakExpr,
    CallExpr, CastExpr, ConstDecl, Crate, EnumDecl, EnumVariant, Expr, ExternFnDecl, FieldAccessExpr, FnDecl, FnSig,
    ForExpr, GenericArg, GenericParam, GenericParamKind, Ident, IfExpr, ImplDecl, IndexExpr, Item, LetExpr, LetStmt,
    LiteralExpr, LoopExpr, MatchArm, MatchExpr, MethodCallExpr, ModDecl, Param, Path, PathExpr, PathSegment, Pattern,
    PatternStructField, ReturnExpr, Stmt, StructDecl, StructExpr, StructExprField, StructFieldDef, TraitDecl,
    TupleExpr, Ty, TyAliasDecl, UnOp, UnaryExpr, UseItem, VariantData, WhileExpr,
};
use crate::parser::ParserError;
use crate::Session;

type PResult<T> = Result<T, ParserError>;

#[derive(Clone, Copy, Default)]
struct Restrictions {
    forbid_struct_expr: bool,
}

pub struct Parser<'a> {
    session: &'a Session,
    tokens: Vec<Token>,
    position: usize,
    restrictions: Restrictions,
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

    fn with_restrictions<T, F>(&mut self, restrictions: Restrictions, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let old = self.restrictions;
        self.restrictions = restrictions;
        let result = f(self);
        self.restrictions = old;
        result
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

    fn parse_delimited<T, F>(&mut self, open: TokenKind, close: TokenKind, mut parse_element: F) -> Vec<T>
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
        loop {
            if self.current_is(&close) || self.at_eof() {
                break;
            }

            match parse_element(self) {
                Ok(element) => elements.push(element),
                Err(err) => {
                    self.emit(err);
                    // Recover to closing delimiter
                    while !self.at_eof() && !self.current_is(&close) {
                        self.advance();
                    }
                    break;
                }
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
        elements
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

            if self.consume(&[separator.clone()]) {
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
        while !self.at_eof() && !self.current().begins_item() {
            self.advance();
        }
        AstNode::err(Item::Err)
    }
}
impl<'a> Parser<'a> {
    pub fn new(session: &'a Session, tokens: Vec<Token>) -> Self {
        Self {
            session,
            tokens,
            position: 0,
            restrictions: Restrictions::default(),
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
        match self.current().kind {
            TokenKind::Keyword(Kw::Fn) => self.parse_fn_item(),
            TokenKind::Keyword(Kw::Struct) => self.parse_struct_item(),
            TokenKind::Keyword(Kw::Enum) => self.parse_enum_item(),
            TokenKind::Keyword(Kw::Trait) => self.parse_trait_item(),
            TokenKind::Keyword(Kw::Mod) => self.parse_mod_item(),
            TokenKind::Keyword(Kw::Impl) => self.parse_impl_item(),
            TokenKind::Keyword(Kw::Extern) => self.parse_extern_fn_item(),
            TokenKind::Keyword(Kw::Const) => self.parse_const_item(),
            TokenKind::Keyword(Kw::Use) => self.parse_use_item(),
            TokenKind::Keyword(Kw::Type) => self.parse_type_alias_item(),
            _ => panic!(),
        }
    }

    fn parse_type_alias(&mut self) -> PResult<AstNode<TyAliasDecl>> {
        let lo = self.current().span;
        self.advance();

        let ident = self.parse_ident()?;
        let generics = self.parse_generic_params()?;

        self.consume(&[TokenKind::Punctuation(Punct::Eq)]);

        let ty = if self.check(&[TokenKind::Punctuation(Punct::Semicolon)]) {
            self.advance();
            None
        } else {
            let ty = self.parse_type()?;
            self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]);
            Some(ty)
        };

        Ok(AstNode::new(
            TyAliasDecl { ident, generics, ty },
            lo.to(self.previous().span),
        ))
    }

    fn parse_extern_fn_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let sig = self.parse_fn_sig()?;
        self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]);

        Ok(AstNode::new(
            Item::ExternFn(ExternFnDecl { sig }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_const_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let ident = self.parse_ident()?;
        let generics = self.parse_generic_params()?;

        let type_annotation = if self.consume(&[TokenKind::Punctuation(Punct::Colon)]) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(&[TokenKind::Punctuation(Punct::Eq)]);

        let expr = self.parse_expression()?;

        self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]);

        Ok(AstNode::new(
            Item::Const(ConstDecl {
                ident,
                generics,
                type_annotation,
                expr,
            }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_use_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let path = self.parse_path()?;
        self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]);

        Ok(AstNode::new(Item::Use(UseItem { path }), lo.to(self.previous().span)))
    }

    fn parse_type_alias_item(&mut self) -> PResult<AstNode<Item>> {
        let ty_alias = self.parse_type_alias()?;
        Ok(AstNode::new(
            Item::TyAlias(ty_alias.node),
            ty_alias.span.to(self.previous().span),
        ))
    }

    fn parse_associated_items(&mut self) -> PResult<Vec<AstNode<AssociatedItem>>> {
        let items = self.parse_delimited(
            TokenKind::OpeningDelimiter(Delimiter::Brace),
            TokenKind::ClosingDelimiter(Delimiter::Brace),
            |p| p.parse_associated_item(),
        );
        Ok(items)
    }

    fn parse_associated_item(&mut self) -> PResult<AstNode<AssociatedItem>> {
        let lo = self.current().span;

        let item = match self.current().kind {
            TokenKind::Keyword(Kw::Fn) => {
                let fn_decl = self.parse_fn_sig()?;
                let body = match self.current().kind {
                    TokenKind::Punctuation(Punct::Semicolon) => {
                        self.advance();
                        None
                    }
                    TokenKind::OpeningDelimiter(Delimiter::Brace) => Some(self.parse_block()?),
                    _ => todo!(),
                };

                AssociatedItem::Fn(fn_decl, body)
            }
            TokenKind::Keyword(Kw::Type) => {
                let ty_alias = self.parse_type_alias()?;
                AssociatedItem::Type(ty_alias)
            }
            _ => todo!(),
        };

        Ok(AstNode::new(item, lo.to(self.previous().span)))
    }

    fn parse_impl_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let generics = self.parse_generic_params()?;
        let self_ty = self.parse_type()?;

        let for_trait = if self.consume(&[TokenKind::Keyword(Kw::For)]) {
            let path = self.parse_path()?;
            Some(path)
        } else {
            None
        };

        let items = self.parse_associated_items()?;

        Ok(AstNode::new(
            Item::Impl(ImplDecl {
                generics,
                self_ty,
                for_trait,
                items,
            }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_trait_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let ident = self.parse_ident()?;
        let generics = self.parse_generic_params()?;
        let items = self.parse_associated_items()?;

        Ok(AstNode::new(
            Item::Trait(TraitDecl { ident, generics, items }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_mod_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let ident = self.parse_ident()?;
        let items = self.parse_delimited(
            TokenKind::OpeningDelimiter(Delimiter::Brace),
            TokenKind::ClosingDelimiter(Delimiter::Brace),
            |p| p.parse_item(),
        );

        Ok(AstNode::new(
            Item::Mod(ModDecl { ident, items }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_enum_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let ident = self.parse_ident()?;
        let generics = self.parse_generic_params()?;

        let variants = self.parse_separated_delimited(
            TokenKind::OpeningDelimiter(Delimiter::Brace),
            TokenKind::ClosingDelimiter(Delimiter::Brace),
            TokenKind::Punctuation(Punct::Comma),
            |p| p.parse_enum_variant(),
        );

        Ok(AstNode::new(
            Item::Enum(EnumDecl {
                ident,
                generics,
                variants,
            }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_enum_variant(&mut self) -> PResult<AstNode<EnumVariant>> {
        let lo = self.current().span;

        let ident = self.parse_ident()?;
        let variant = self.parse_variant_data()?;

        Ok(AstNode::new(
            EnumVariant { ident, data: variant },
            lo.to(self.previous().span),
        ))
    }

    fn parse_variant_data(&mut self) -> PResult<AstNode<VariantData>> {
        let lo = self.previous().span;

        let variant = match self.current().kind {
            TokenKind::OpeningDelimiter(Delimiter::Brace) => {
                let fields = self.parse_separated_delimited(
                    TokenKind::OpeningDelimiter(Delimiter::Brace),
                    TokenKind::ClosingDelimiter(Delimiter::Brace),
                    TokenKind::Punctuation(Punct::Comma),
                    |p| p.parse_struct_item_field(),
                );
                VariantData::Struct { fields }
            }
            TokenKind::OpeningDelimiter(Delimiter::Paren) => {
                let types = self.parse_separated_delimited(
                    TokenKind::OpeningDelimiter(Delimiter::Paren),
                    TokenKind::ClosingDelimiter(Delimiter::Paren),
                    TokenKind::Punctuation(Punct::Comma),
                    |p| p.parse_type(),
                );
                VariantData::Tuple { types }
            }
            TokenKind::Punctuation(Punct::Semicolon) => VariantData::Unit,
            _ => VariantData::Unit,
        };
        Ok(AstNode::new(variant, lo.to(self.previous().span)))
    }
    fn parse_struct_item(&mut self) -> PResult<AstNode<Item>> {
        let lo = self.current().span;
        self.advance();

        let ident = self.parse_ident()?;
        let generics = self.parse_generic_params()?;
        let variant = self.parse_variant_data()?;

        if !matches!(variant.node, VariantData::Struct { .. }) {
            self.consume(&[TokenKind::Punctuation(Punct::Semicolon)]);
        }

        Ok(AstNode::new(
            Item::Struct(StructDecl {
                ident,
                generics,
                data: variant,
            }),
            lo.to(self.previous().span),
        ))
    }

    fn parse_struct_item_field(&mut self) -> PResult<AstNode<StructFieldDef>> {
        let lo = self.current().span;

        let ident = self.parse_ident()?;
        self.consume(&[TokenKind::Punctuation(Punct::Colon)]);
        let ty = self.parse_type()?;

        Ok(AstNode::new(
            StructFieldDef {
                ident,
                type_annotation: ty,
            },
            lo.to(self.previous().span),
        ))
    }

    /// fn a(b: i32) -> i32 { b }
    /// starts at 'fn', ends after the block
    fn parse_fn_item(&mut self) -> PResult<AstNode<Item>> {
        let fn_decl = self.parse_fn_decl()?;

        Ok(AstNode::new(
            Item::Fn(fn_decl.node),
            fn_decl.span.to(self.previous().span),
        ))
    }

    fn parse_fn_decl(&mut self) -> PResult<AstNode<FnDecl>> {
        let lo = self.current().span;

        let sig = self.parse_fn_sig()?;
        let body = self.parse_block()?;

        Ok(AstNode::new(FnDecl { sig, body }, lo.to(self.previous().span)))
    }

    // starts at the `fn` keyword, ends before the block
    fn parse_fn_sig(&mut self) -> PResult<AstNode<FnSig>> {
        let lo = self.current().span;
        self.advance();

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

        let mut pattern = self.parse_pattern_no_or()?;

        if self.check(&[TokenKind::Punctuation(Punct::Pipe)]) {
            let mut patterns = vec![pattern];
            while self.consume(&[TokenKind::Punctuation(Punct::Pipe)]) {
                patterns.push(self.parse_pattern_no_or()?);
            }
            pattern = AstNode::new(Pattern::Or(patterns), lo.to(self.previous().span));
        }

        Ok(pattern)
    }

    fn parse_pattern_no_or(&mut self) -> PResult<AstNode<Pattern>> {
        let lo = self.current().span;

        let pattern = match &self.current().kind {
            TokenKind::Punctuation(Punct::Underscore) => {
                self.advance();
                Pattern::Wildcard
            }
            TokenKind::Ident(_) => {
                let path = self.parse_path()?;

                if self.check(&[TokenKind::OpeningDelimiter(Delimiter::Brace)]) {
                    // Struct pattern: Path { field: pattern, ... }
                    let fields = self.parse_separated_delimited(
                        TokenKind::OpeningDelimiter(Delimiter::Brace),
                        TokenKind::ClosingDelimiter(Delimiter::Brace),
                        TokenKind::Punctuation(Punct::Comma),
                        |p| p.parse_pattern_struct_field(),
                    );
                    Pattern::Struct(path, fields)
                } else if self.check(&[TokenKind::OpeningDelimiter(Delimiter::Paren)]) {
                    // TupleStruct pattern: Path(pattern, ...)
                    let patterns = self.parse_separated_delimited(
                        TokenKind::OpeningDelimiter(Delimiter::Paren),
                        TokenKind::ClosingDelimiter(Delimiter::Paren),
                        TokenKind::Punctuation(Punct::Comma),
                        |p| p.parse_pattern(),
                    );
                    Pattern::TupleStruct(path, patterns)
                } else {
                    Pattern::Path(path)
                }
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
            TokenKind::Literal(_) => {
                let expr = self.parse_expression()?;
                Pattern::Expr(Box::new(expr))
            }
            _ => panic!("Expected Pattern"),
        };

        Ok(AstNode::new(pattern, lo.to(self.previous().span)))
    }

    fn parse_pattern_struct_field(&mut self) -> PResult<AstNode<PatternStructField>> {
        let lo = self.current().span;

        let ident = self.parse_ident()?;
        self.consume(&[TokenKind::Punctuation(Punct::Colon)]);
        let pattern = self.parse_pattern()?;

        Ok(AstNode::new(
            PatternStructField { ident, pattern },
            lo.to(self.previous().span),
        ))
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

        let is_const = self.consume(&[TokenKind::Keyword(Kw::Const)]);
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
            GenericParam { ident, bounds, kind },
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

        Ok(AstNode::new(PathSegment { ident, args }, lo.to(self.previous().span)))
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

        let generic_arg = if self.consume(&[TokenKind::Keyword(Kw::Const)]) {
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
            TokenKind::Keyword(Kw::Fn) => {
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
                Ok(AstNode::new(Ident::new(ident.into()), lo.to(self.previous().span)))
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

        while !self.consume(&[TokenKind::ClosingDelimiter(Delimiter::Brace), TokenKind::EOF]) {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
        }
        Ok(AstNode::new(BlockExpr { stmts }, lo.to(self.previous().span)))
    }

    fn parse_statement(&mut self) -> PResult<AstNode<Stmt>> {
        let lo = self.current().span;

        let stmt = match self.current().kind {
            TokenKind::Keyword(Kw::Let) => self.parse_let_stmt()?,

            _ if self.current().begins_item() => {
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
            let (left_bp, right_bp) = self.current_precedence();
            if left_bp < min_prec {
                break;
            }

            match self.current().kind.clone() {
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
                    );
                    continue;
                }

                TokenKind::Punctuation(Punct::Dot) => {
                    self.advance();
                    let ident = self.parse_ident()?;

                    if self.check(&[TokenKind::OpeningDelimiter(Delimiter::Paren)]) {
                        let args = self.parse_separated_delimited(
                            TokenKind::OpeningDelimiter(Delimiter::Paren),
                            TokenKind::ClosingDelimiter(Delimiter::Paren),
                            TokenKind::Punctuation(Punct::Comma),
                            |p| p.parse_expression(),
                        );

                        let name = AstNode::new(
                            PathSegment {
                                ident: ident.clone(),
                                args: vec![],
                            },
                            ident.span.to(self.previous().span),
                        );

                        lhs = AstNode::new(
                            Expr::MethodCall(MethodCallExpr {
                                name,
                                receiver: Box::new(lhs.clone()),
                                arguments: args,
                            }),
                            lhs.span.to(self.previous().span),
                        );
                    } else {
                        lhs = AstNode::new(
                            Expr::FieldAccess(FieldAccessExpr {
                                target: Box::new(lhs.clone()),
                                field: ident,
                            }),
                            lhs.span.to(self.previous().span),
                        );
                    }
                    continue;
                }

                TokenKind::OpeningDelimiter(Delimiter::Bracket) => {
                    self.advance();
                    let index_expr = self.parse_expression()?;
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
                            index: Box::new(index_expr),
                        }),
                        lhs.span.to(self.previous().span),
                    );
                    continue;
                }

                TokenKind::Keyword(Kw::As) => {
                    self.advance();
                    let ty = self.parse_type()?;
                    lhs = AstNode::new(
                        Expr::Cast(CastExpr {
                            expr: Box::new(lhs.clone()),
                            ty,
                        }),
                        lhs.span.to(self.previous().span),
                    );
                    continue;
                }

                _ if self.current().kind.is_infix_op() => {
                    let op_token = self.current().clone();
                    self.advance();

                    let rhs = self.parse_expr_with_precedence(right_bp)?;
                    lhs = self.make_infix_expr(lhs, op_token, rhs)?;
                    continue;
                }

                _ => break,
            }
        }

        Ok(lhs)
    }

    fn current_precedence(&self) -> (u8, u8) {
        match &self.current().kind {
            TokenKind::OpeningDelimiter(Delimiter::Paren)
            | TokenKind::OpeningDelimiter(Delimiter::Bracket)
            | TokenKind::Punctuation(Punct::Dot) => (100, 101),

            TokenKind::Keyword(Kw::As) => (12, 13),

            TokenKind::Punctuation(Punct::Star)
            | TokenKind::Punctuation(Punct::Slash)
            | TokenKind::Punctuation(Punct::Percent) => (11, 12),

            TokenKind::Punctuation(Punct::Plus) | TokenKind::Punctuation(Punct::Minus) => (10, 11),

            TokenKind::Punctuation(Punct::Less)
            | TokenKind::Punctuation(Punct::LessEq)
            | TokenKind::Punctuation(Punct::Greater)
            | TokenKind::Punctuation(Punct::GreaterEq) => (5, 6),

            TokenKind::Punctuation(Punct::EqEq) | TokenKind::Punctuation(Punct::NotEq) => (4, 5),

            TokenKind::Punctuation(Punct::And) => (3, 4),
            TokenKind::Punctuation(Punct::Or) => (2, 3),

            TokenKind::Punctuation(Punct::Eq)
            | TokenKind::Punctuation(Punct::PlusEq)
            | TokenKind::Punctuation(Punct::MinusEq)
            | TokenKind::Punctuation(Punct::StarEq)
            | TokenKind::Punctuation(Punct::SlashEq)
            | TokenKind::Punctuation(Punct::PercentEq) => (1, 0),

            _ => (0, 0),
        }
    }

    fn make_infix_expr(&mut self, lhs: AstNode<Expr>, op: Token, rhs: AstNode<Expr>) -> PResult<AstNode<Expr>> {
        use Punct::*;
        use TokenKind::*;

        let span = lhs.span.to(rhs.span);

        match op.kind {
            Punctuation(Eq) => Ok(AstNode::new(
                Expr::Assign(AssignExpr {
                    target: Box::new(lhs),
                    value: Box::new(rhs),
                }),
                span,
            )),
            Punctuation(Plus)
            | Punctuation(Minus)
            | Punctuation(Star)
            | Punctuation(Slash)
            | Punctuation(Percent)
            | Punctuation(And)
            | Punctuation(Or)
            | Punctuation(EqEq)
            | Punctuation(Less)
            | Punctuation(LessEq)
            | Punctuation(Greater)
            | Punctuation(GreaterEq)
            | Punctuation(NotEq) => Ok(AstNode::new(
                Expr::Binary(BinaryExpr {
                    operator: AstNode::new(BinOp::try_from(&op).unwrap(), op.span),
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                }),
                span,
            )),
            Punctuation(PlusEq)
            | Punctuation(MinusEq)
            | Punctuation(StarEq)
            | Punctuation(SlashEq)
            | Punctuation(PercentEq) => Ok(AstNode::new(
                Expr::AssignOp(AssignOpExpr {
                    target: Box::new(lhs),
                    op: AstNode::new(AssignOp::try_from(&op).unwrap(), op.span),
                    value: Box::new(rhs),
                }),
                span,
            )),

            _ => todo!(),
        }
    }

    fn parse_prefix(&mut self) -> PResult<AstNode<Expr>> {
        let lo = self.current().span;

        let Ok(op) = UnOp::try_from(self.current()) else {
            return self.parse_atom();
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
            TokenKind::Keyword(Kw::True) => {
                self.advance();
                Expr::Literal(LiteralExpr::Bool(true))
            }
            TokenKind::Keyword(Kw::False) => {
                self.advance();
                Expr::Literal(LiteralExpr::Bool(false))
            }
            TokenKind::Keyword(Kw::Unit) => {
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
                if !self.restrictions.forbid_struct_expr && self.check(&[TokenKind::OpeningDelimiter(Delimiter::Brace)])
                {
                    Expr::Struct(self.parse_struct_expr(path)?.node)
                } else {
                    Expr::Path(PathExpr { path })
                }
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
            TokenKind::Keyword(Kw::If) => {
                let if_expr = self.parse_if_expr()?;
                Expr::If(if_expr.node)
            }
            TokenKind::Keyword(Kw::While) => {
                let while_expr = self.parse_while_expr()?;
                Expr::While(while_expr.node)
            }
            TokenKind::Keyword(Kw::Loop) => {
                let loop_expr = self.parse_loop_expr()?;
                Expr::Loop(loop_expr.node)
            }
            TokenKind::Keyword(Kw::For) => {
                let for_expr = self.parse_for_expr()?;
                Expr::For(for_expr.node)
            }
            TokenKind::Keyword(Kw::Match) => {
                let match_expr = self.parse_match_expr()?;
                Expr::Match(match_expr.node)
            }
            TokenKind::Keyword(Kw::Break) => {
                let break_expr = self.parse_break_expr()?;
                Expr::Break(break_expr.node)
            }
            TokenKind::Keyword(Kw::Continue) => {
                self.advance();
                Expr::Continue
            }
            TokenKind::Keyword(Kw::Let) => {
                let let_expr = self.parse_let_expr()?;
                Expr::Let(let_expr.node)
            }
            TokenKind::Keyword(Kw::Return) => {
                self.advance();
                let expr = if self.current().can_begin_expr() {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                Expr::Return(ReturnExpr { value: expr })
            }
            _ => {
                panic!("{}", self.current().kind)
            }
        };

        Ok(AstNode::new(atom, lo.to(self.previous().span)))
    }

    fn parse_if_expr(&mut self) -> PResult<AstNode<IfExpr>> {
        let lo = self.current().span;
        self.advance();

        let condition = self.with_restrictions(
            Restrictions {
                forbid_struct_expr: true,
            },
            |p| p.parse_expression(),
        )?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.consume(&[TokenKind::Keyword(Kw::Else)]) {
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
        let lo = self.current().span;
        self.advance();

        let condition = self.with_restrictions(
            Restrictions {
                forbid_struct_expr: true,
            },
            |p| p.parse_expression(),
        )?;
        let body = self.parse_block()?;

        Ok(AstNode::new(
            WhileExpr {
                condition: Box::new(condition),
                body: Box::new(body),
            },
            lo.to(self.previous().span),
        ))
    }

    fn parse_loop_expr(&mut self) -> PResult<AstNode<LoopExpr>> {
        let lo = self.current().span;
        self.advance();

        let body = self.parse_block()?;

        Ok(AstNode::new(
            LoopExpr { body: Box::new(body) },
            lo.to(self.previous().span),
        ))
    }

    fn parse_for_expr(&mut self) -> PResult<AstNode<ForExpr>> {
        let lo = self.current().span;
        self.advance();

        let pattern = self.parse_pattern()?;

        self.consume(&[TokenKind::Keyword(Kw::In)]);

        let iterator = self.with_restrictions(
            Restrictions {
                forbid_struct_expr: true,
            },
            |p| p.parse_expression(),
        )?;
        let body = self.parse_block()?;

        Ok(AstNode::new(
            ForExpr {
                pattern,
                iterator: Box::new(iterator),
                body: Box::new(body),
            },
            lo.to(self.previous().span),
        ))
    }

    fn parse_match_expr(&mut self) -> PResult<AstNode<MatchExpr>> {
        let lo = self.current().span;
        self.advance();

        let expr = self.with_restrictions(
            Restrictions {
                forbid_struct_expr: true,
            },
            |p| p.parse_expression(),
        )?;

        let arms = self.parse_separated_delimited(
            TokenKind::OpeningDelimiter(Delimiter::Brace),
            TokenKind::ClosingDelimiter(Delimiter::Brace),
            TokenKind::Punctuation(Punct::Comma),
            |p| p.parse_match_arm(),
        );
        Ok(AstNode::new(
            MatchExpr {
                value: Box::new(expr),
                arms,
            },
            lo.to(self.previous().span),
        ))
    }

    fn parse_match_arm(&mut self) -> PResult<AstNode<MatchArm>> {
        let lo = self.current().span;
        let pattern = self.parse_pattern()?;

        self.consume(&[TokenKind::Punctuation(Punct::FatArrow)]);
        let expr = self.parse_expression()?;

        Ok(AstNode::new(
            MatchArm {
                pattern,
                body: Box::new(expr),
            },
            lo.to(self.previous().span),
        ))
    }

    fn parse_break_expr(&mut self) -> PResult<AstNode<BreakExpr>> {
        let lo = self.current().span;
        self.advance();

        let expr = if self.current().can_begin_expr() {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        Ok(AstNode::new(BreakExpr { expr }, lo.to(self.previous().span)))
    }

    fn parse_let_expr(&mut self) -> PResult<AstNode<LetExpr>> {
        let lo = self.current().span;
        self.advance();

        let pattern = self.parse_pattern()?;
        self.consume(&[TokenKind::Punctuation(Punct::Eq)]);

        let expr = self.parse_expression()?;

        Ok(AstNode::new(
            LetExpr {
                pattern,
                value: Box::new(expr),
            },
            lo.to(self.previous().span),
        ))
    }

    fn parse_struct_expr(&mut self, path: AstNode<Path>) -> PResult<AstNode<StructExpr>> {
        let lo = path.span;

        let fields = self.parse_separated_delimited(
            TokenKind::OpeningDelimiter(Delimiter::Brace),
            TokenKind::ClosingDelimiter(Delimiter::Brace),
            TokenKind::Punctuation(Punct::Comma),
            |p| p.parse_struct_expr_field(),
        );

        Ok(AstNode::new(
            StructExpr { name: path, fields },
            lo.to(self.previous().span),
        ))
    }

    fn parse_struct_expr_field(&mut self) -> PResult<AstNode<StructExprField>> {
        let lo = self.current().span;

        let ident = self.parse_ident()?;
        self.consume(&[TokenKind::Punctuation(Punct::Colon)]);
        let expr = self.parse_expression()?;

        Ok(AstNode::new(
            StructExprField {
                ident,
                expr: Box::new(expr),
            },
            lo.to(self.previous().span),
        ))
    }
}
