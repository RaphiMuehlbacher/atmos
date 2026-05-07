use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::{Crate, HirNode, Item};
use crate::resolver::DefId;
use crate::type_checker::ty;
use crate::type_checker::ty::GenericArg;
use crate::Session;
use std::collections::HashMap;

pub struct TypeCollector<'hir> {
    session: &'hir Session,
    items: HashMap<DefId, ty::Ty>,
    hir_krate: &'hir Crate,
}

impl<'hir> TypeCollector<'hir> {
    pub fn new(session: &'hir Session, hir_krate: &'hir Crate) -> Self {
        Self {
            session,
            hir_krate,
            items: HashMap::new(),
        }
    }

    pub fn collect_items(&mut self) {
        for item in &self.hir_krate.items {
            self.collect_item(item);
        }
    }

    fn collect_item(&mut self, item: &HirNode<Item>) {
        match &item.node {
            Item::Fn(fn_decl) => {
                let generic_args = fn_decl
                    .sig
                    .node
                    .generics
                    .iter()
                    .map(|arg| GenericArg::Type(ty::Ty::GenericParam(arg.node.def_id)))
                    .collect();

                self.items
                    .insert(fn_decl.def_id, ty::Ty::Fn(fn_decl.def_id, generic_args));
            }
            Item::Struct(struct_decl) => {
                let generic_args = struct_decl
                    .generics
                    .iter()
                    .map(|arg| GenericArg::Type(ty::Ty::GenericParam(arg.node.def_id)))
                    .collect();

                self.items
                    .insert(struct_decl.def_id, ty::Ty::Fn(struct_decl.def_id, generic_args));
            }
            Item::Enum(enum_decl) => {
                let generic_args = enum_decl
                    .generics
                    .iter()
                    .map(|arg| GenericArg::Type(ty::Ty::GenericParam(arg.node.def_id)))
                    .collect();

                self.items
                    .insert(enum_decl.def_id, ty::Ty::Fn(enum_decl.def_id, generic_args));
            }
            Item::Trait(_) => {}
            Item::Mod(_) => {}
            Item::Impl(_) => {}
            Item::ExternFn(fn_decl) => {
                let generic_args = fn_decl
                    .sig
                    .node
                    .generics
                    .iter()
                    .map(|arg| GenericArg::Type(ty::Ty::GenericParam(arg.node.def_id)))
                    .collect();

                self.items
                    .insert(fn_decl.def_id, ty::Ty::Fn(fn_decl.def_id, generic_args));
            }
            Item::Const(const_item) => {
                self.items.insert(const_item.def_id, self.lower_ty(const_item));
            }
            Item::TyAlias(_) => {}
        }
    }

    fn lower_ty(&self, hir_ty: &HirNode<hir::Ty>) -> ty::Ty {
        match &hir_ty.node {
            hir::Ty::Path(_) => todo!(),
            hir::Ty::Array(_, _) => todo!(),
            hir::Ty::Ptr(ty) => ty::Ty::Ptr(Box::new(self.lower_ty(ty))),
            hir::Ty::Fn(params, return_ty) => {
                let params = params.iter().map(|p| self.lower_ty(p)).collect();
                let return_ty = return_ty
                    .as_ref()
                    .map_or(ty::Ty::Unit, |return_ty| self.lower_ty(return_ty));
                ty::Ty::FnPtr(params, Box::new(return_ty))
            }
            hir::Ty::Tuple(types) => {
                if types.is_empty() {
                    ty::Ty::Unit
                } else {
                    ty::Ty::Tuple(types.iter().map(|ty| self.lower_ty(ty)).collect())
                }
            }
            hir::Ty::Err => ty::Ty::Err,
        }
    }
}
