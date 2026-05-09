use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::{Crate, HirId, HirNode, Item, Node, Path};
use crate::resolver::defs::DefKind;
use crate::resolver::ribs::Res;
use crate::resolver::DefId;
use crate::type_checker::ty;
use crate::type_checker::ty::{Const, GenericArg, GenericArgs};
use crate::Session;
use std::collections::HashMap;

pub struct TypeCollector<'hir> {
    session: &'hir Session,
    items: HashMap<DefId, ty::Ty>,
    hir_krate: &'hir Crate,
    hir_nodes: &'hir HashMap<HirId, Node>,
}

impl<'hir> TypeCollector<'hir> {
    pub fn new(session: &'hir Session, hir_krate: &'hir Crate, hir_nodes: &'hir HashMap<HirId, Node>) -> Self {
        Self {
            session,
            hir_krate,
            hir_nodes,
            items: HashMap::new(),
        }
    }

    pub fn collect_items(&mut self) {
        for item in &self.hir_krate.items {
            self.collect_item_def(item);
        }
    }

    fn collect_item_def(&mut self, item: &HirNode<Item>) {
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
                    .insert(struct_decl.def_id, ty::Ty::Struct(struct_decl.def_id, generic_args));
            }
            Item::Enum(enum_decl) => {
                let generic_args = enum_decl
                    .generics
                    .iter()
                    .map(|arg| GenericArg::Type(ty::Ty::GenericParam(arg.node.def_id)))
                    .collect();

                self.items
                    .insert(enum_decl.def_id, ty::Ty::Enum(enum_decl.def_id, generic_args));
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
                self.items.insert(const_item.def_id, self.lower_ty(&const_item.ty));
            }
            Item::TyAlias(_) => {
                todo!()
            }
        }
    }

    fn lower_generic_args(&self, generic_args: &Vec<HirNode<hir::GenericArg>>) -> GenericArgs {
        let mut lowered_args = vec![];
        for arg in generic_args {
            let lowered_arg = match &arg.node {
                hir::GenericArg::Type(ty) => GenericArg::Type(self.lower_ty(&ty)),
                hir::GenericArg::Const(expr) => GenericArg::Const(Const::Expr(expr.node.clone())),
            };
            lowered_args.push(lowered_arg);
        }
        lowered_args
    }

    fn lower_ty(&self, hir_ty: &HirNode<hir::Ty>) -> ty::Ty {
        match &hir_ty.node {
            hir::Ty::Path(path) => match &path.node {
                Path::Resolved { res, segments } => {
                    let Res::Def(def_id, def_kind) = res else { panic!() };

                    match def_kind {
                        DefKind::Struct => {
                            let segment = segments.last().unwrap();
                            let generic_args = self.lower_generic_args(&segment.node.args);
                            ty::Ty::Struct(def_id.clone(), generic_args)
                        }
                        DefKind::Enum => {
                            let segment = segments.last().unwrap();
                            let generic_args = self.lower_generic_args(&segment.node.args);
                            ty::Ty::Enum(def_id.clone(), generic_args)
                        }
                        DefKind::TypeAlias => todo!(),
                        DefKind::Function => {
                            let segment = segments.last().unwrap();
                            let generic_args = self.lower_generic_args(&segment.node.args);
                            ty::Ty::Fn(def_id.clone(), generic_args)
                        }
                        DefKind::AssocFn => {
                            let segment = segments.last().unwrap();
                            let generic_args = self.lower_generic_args(&segment.node.args);
                            ty::Ty::Fn(def_id.clone(), generic_args)
                        }
                        DefKind::ExternFn => {
                            let segment = segments.last().unwrap();
                            let generic_args = self.lower_generic_args(&segment.node.args);
                            ty::Ty::Fn(def_id.clone(), generic_args)
                        }
                        DefKind::TypeParam => ty::Ty::GenericParam(def_id.clone()),
                        DefKind::StructField
                        | DefKind::EnumVariant
                        | DefKind::Trait
                        | DefKind::Mod
                        | DefKind::Impl
                        | DefKind::Use
                        | DefKind::Const
                        | DefKind::AssocTypeAlias => ty::Ty::Err,
                    }
                }
                Path::Unresolved {
                    res,
                    resolved_segments,
                    unresolved_segments,
                } => todo!(),
            },
            hir::Ty::Array(elem_ty, _) => {
                // TODO: Handle const expressions in array types
                ty::Ty::Slice(Box::new(self.lower_ty(elem_ty)))
            }
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
