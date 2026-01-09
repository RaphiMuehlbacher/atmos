use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::HirNode;
use crate::parser::ast;
use crate::parser::ast::AstNode;
use crate::resolver::defs::DefinitionMap;
use crate::Session;

pub struct AstLowerer<'ast> {
    ast: &'ast ast::Crate,
    session: &'ast Session,
    defs: &'ast DefinitionMap,
}

impl<'ast> AstLowerer<'ast> {
    pub fn new(session: &'ast Session, defs: &'ast DefinitionMap, ast: &'ast ast::Crate) -> Self {
        Self { ast, session, defs }
    }

    pub fn lower(&mut self) -> hir::Crate {
        let items = self.ast.items.iter().map(|item| self.lower_item(item)).collect();

        hir::Crate {
            items,
            span: self.ast.span,
        }
    }

    fn lower_item(&mut self, item: &AstNode<ast::Item>) -> HirNode<hir::Item> {
        let hir_item = match &item.node {
            ast::Item::Fn(fn_item) => {
                let def_id = *self.defs.ast_to_def.get(&item.ast_id).unwrap();

                let sig = self.lower_fn_sig(&fn_item.sig);
                let body = self.lower_block_expr(&fn_item.body);

                hir::Item::Fn(hir::FnDecl { def_id, sig, body })
            }
            ast::Item::Struct(struct_item) => todo!(),
            ast::Item::Enum(enum_item) => todo!(),
            ast::Item::Trait(trait_item) => todo!(),
            ast::Item::Mod(mod_item) => todo!(),
            ast::Item::Impl(impl_item) => todo!(),
            ast::Item::ExternFn(extern_fn_item) => todo!(),
            ast::Item::Const(const_item) => todo!(),
            ast::Item::Use(use_item) => todo!(),
            ast::Item::TyAlias(ty_alias_item) => todo!(),
            ast::Item::Err => unreachable!(),
        };

        HirNode::new(hir_item, item.span)
    }

    fn lower_fn_sig(&mut self, sig: &AstNode<ast::FnSig>) -> HirNode<hir::FnSig> {
        let ident = sig.node.ident.clone().into();
        let generics = sig
            .node
            .generics
            .iter()
            .map(|generic| self.lower_generic_param(generic))
            .collect();

        let params = sig.node.params.iter().map(|param| self.lower_param(param)).collect();
        let return_ty = sig.node.return_ty.as_ref().map(|return_ty| self.lower_type(return_ty));

        HirNode::new(
            hir::FnSig {
                ident,
                generics,
                params,
                return_ty,
            },
            sig.span,
        )
    }

    fn lower_param(&mut self, param: &AstNode<ast::Param>) -> HirNode<hir::Param> {
        let pattern = self.lower_pattern(&param.node.pattern);
        let type_annotation = self.lower_type(&param.node.type_annotation);
        HirNode::new(
            hir::Param {
                pattern,
                type_annotation,
            },
            param.span,
        )
    }

    fn lower_generic_param(&mut self, generic_param: &AstNode<ast::GenericParam>) -> HirNode<hir::GenericParam> {
        let def_id = *self.defs.ast_to_def.get(&generic_param.ast_id).unwrap();
        let ident = generic_param.node.ident.clone().into();
        let bounds = generic_param
            .node
            .bounds
            .iter()
            .map(|bound| self.lower_path(&bound))
            .collect();
        let kind = match &generic_param.node.kind {
            ast::GenericParamKind::Const(ty) => hir::GenericParamKind::Const(self.lower_type(&ty)),
            ast::GenericParamKind::Type => hir::GenericParamKind::Type,
        };

        HirNode::new(
            hir::GenericParam {
                def_id,
                ident,
                bounds,
                kind,
            },
            generic_param.span,
        )
    }

    fn lower_path(&mut self, path: &AstNode<ast::Path>) -> HirNode<hir::Path> {
        let segments = path
            .node
            .segments
            .iter()
            .map(|segment| self.lower_segment(&segment))
            .collect();
        let res = self.defs.get_resolution(path.ast_id).unwrap().clone();
        HirNode::new(hir::Path { segments, res }, path.span)
    }

    fn lower_segment(&mut self, segment: &AstNode<ast::PathSegment>) -> HirNode<hir::PathSegment> {
        let ident = segment.node.ident.clone().into();
        let args = segment
            .node
            .args
            .iter()
            .map(|arg| self.lower_generic_arg(arg))
            .collect();

        HirNode::new(hir::PathSegment { ident, args }, segment.span)
    }

    fn lower_generic_arg(&mut self, generic_arg: &AstNode<ast::GenericArg>) -> HirNode<hir::GenericArg> {
        let arg = match &generic_arg.node {
            ast::GenericArg::Type(ty) => hir::GenericArg::Type(self.lower_type(ty)),
            ast::GenericArg::Const(expr) => hir::GenericArg::Const(Box::new(self.lower_expr(expr))),
        };
        HirNode::new(arg, generic_arg.span)
    }

    fn lower_type(&mut self, ty: &AstNode<ast::Ty>) -> HirNode<hir::Ty> {
        let hir_ty = match &ty.node {
            ast::Ty::Path(path) => hir::Ty::Path(self.lower_path(path)),
            ast::Ty::Array(ty, expr) => hir::Ty::Array(Box::new(self.lower_type(ty)), Box::new(self.lower_expr(expr))),
            ast::Ty::Ptr(ty) => hir::Ty::Ptr(Box::new(self.lower_type(ty))),
            ast::Ty::Fn(param_tys, return_ty) => {
                let param_tys = param_tys.iter().map(|param_ty| self.lower_type(param_ty)).collect();
                let return_ty = return_ty
                    .as_ref()
                    .as_ref()
                    .map(|return_ty| Box::new(self.lower_type(return_ty)));
                hir::Ty::Fn(param_tys, return_ty)
            }
            ast::Ty::Tuple(types) => {
                let types = types.iter().map(|ty| self.lower_type(ty)).collect();
                hir::Ty::Tuple(types)
            }
            ast::Ty::Paren(ty) => self.lower_type(ty).node,
        };
        HirNode::new(hir_ty, ty.span)
    }

    fn lower_expr(&mut self, expr: &AstNode<ast::Expr>) -> HirNode<hir::Expr> {
        todo!()
    }

    fn lower_block_expr(&mut self, block: &AstNode<ast::BlockExpr>) -> HirNode<hir::BlockExpr> {
        todo!()
    }

    fn lower_pattern(&mut self, pattern: &AstNode<ast::Pattern>) -> HirNode<hir::Pattern> {
        let pat = match &pattern.node {
            ast::Pattern::Wildcard => hir::Pattern::Wildcard,
            ast::Pattern::Or(patterns) => {
                hir::Pattern::Or(patterns.iter().map(|pat| self.lower_pattern(pat)).collect())
            }
            ast::Pattern::Path(path) => hir::Pattern::Path(self.lower_path(path)),
            ast::Pattern::Struct(path, struct_fields) => {
                let path = self.lower_path(path);
                let struct_fields = struct_fields
                    .iter()
                    .map(|pat| self.lower_pattern_struct_field(pat))
                    .collect();
                hir::Pattern::Struct(path, struct_fields)
            }
            ast::Pattern::TupleStruct(path, patterns) => {
                let path = self.lower_path(path);
                let patterns = patterns.iter().map(|pattern| self.lower_pattern(pattern)).collect();
                hir::Pattern::TupleStruct(path, patterns)
            }
            ast::Pattern::Tuple(patterns) => {
                let patterns = patterns.iter().map(|pattern| self.lower_pattern(pattern)).collect();
                hir::Pattern::Tuple(patterns)
            }
            ast::Pattern::Expr(expr) => hir::Pattern::Expr(Box::new(self.lower_expr(expr))),
            ast::Pattern::Paren(pattern) => self.lower_pattern(pattern).node,
        };

        HirNode::new(pat, pattern.span)
    }
    fn lower_pattern_struct_field(
        &mut self,
        struct_field: &AstNode<ast::PatternStructField>,
    ) -> HirNode<hir::PatternStructField> {
        let ident = struct_field.node.ident.clone().into();
        let pattern = self.lower_pattern(&struct_field.node.pattern);

        HirNode::new(hir::PatternStructField { ident, pattern }, struct_field.span)
    }
}
