use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::{HirNode};
use crate::parser::ast::{AstNode};
use crate::parser::{ast, AstId};
use crate::resolver::defs::DefId;
use crate::Session;
use std::collections::HashMap;

pub struct AstLowerer<'ast> {
    ast: &'ast ast::Crate,
    session: &'ast Session,
    ast_to_def: &'ast HashMap<AstId, DefId>,
}

impl<'ast> AstLowerer<'ast> {
    pub fn new(session: &'ast Session, ast_to_def: &'ast HashMap<AstId, DefId>, ast: &'ast ast::Crate) -> Self {
        Self {
            ast,
            session,
            ast_to_def,
        }
    }

    pub fn lower(&mut self) -> hir::Crate {
        let items = self.ast.items.iter().map(|item| self.lower_item(item)).collect();

        hir::Crate {
            items,
            span: self.ast.span,
        }
    }

    fn lower_item(&mut self, item: &AstNode<ast::Item>) -> HirNode<hir::Item> {
        let hir_item = match item.node {
            ast::Item::Fn(fn_item) => {
                let def_id = self.ast_to_def.get(&item.ast_id).unwrap();

                let sig = self.lower_fn_sig(&fn_item.sig);
                let body = todo!();

                hir::FnDecl { def_id, sig, body}
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
        let generics = sig.node.generics.iter().map(|generic| self.lower_generic_param(generic)) .collect()
        let fn_sig = FnSig {
            ident: sig.node.ident.into(),
            generics,
        };
        HirNode::new(hir::Fn)
    }

    fn lower_generic_param(&mut self, generic_param: &AstNode<ast::GenericParam>) -> HirNode<hir::GenericParam> {
        let def_id = *self.ast_to_def.get(&generic_param.ast_id).unwrap();
        let ident = generic_param.node.ident.clone().into();
        let bounds = generic_param.node.bounds.iter().map(|bound| self.lower_path(&bound)).collect();
        let kind = match &generic_param.node.kind {
            ast::GenericParamKind::Const(ty) => hir::GenericParamKind::Const(self.lower_type(&ty)),
            ast::GenericParamKind::Type => hir::GenericParamKind::Type,
        };

        HirNode::new(hir::GenericParam {def_id, ident, bounds, kind}, generic_param.span)
    }

    fn lower_path(&mut self, path: &AstNode<ast::Path>) -> HirNode<hir::Path> {
        let segments = path.node.segments.iter().map(|segment| self.lower_segment(&segment)).collect();
        let res =
        HirNode::new(hir::Path {segments, res}, path.span)
    }

    fn lower_segment(&mut self, path: &AstNode<ast::PathSegment>) -> HirNode<hir::PathSegment> {
        todo!()
    }

    fn lower_type(&mut self, ty: &AstNode<ast::Ty>) -> HirNode<hir::Ty> {
        todo!()
    }
}
