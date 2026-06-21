use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::{Crate, GenericParamKind, HirId, HirNode, Item, Node};
use crate::resolver::DefId;
use crate::type_checker::ty;
pub(crate) use crate::type_checker::ty::CollectedTypes;
use crate::type_checker::ty::{GenericArg, GenericArgs, Generics};
use crate::Session;
use std::collections::HashMap;

pub struct TypeCollector<'hir> {
    session: &'hir Session,
    hir_krate: &'hir Crate,
    hir_nodes: &'hir HashMap<HirId, Node>,
    def_to_hir: &'hir HashMap<DefId, HirId>,

    collected_types: CollectedTypes,
}

impl<'hir> TypeCollector<'hir> {
    pub fn new(
        session: &'hir Session,
        hir_krate: &'hir Crate,
        hir_nodes: &'hir HashMap<HirId, Node>,
        def_to_hir: &'hir HashMap<DefId, HirId>,
    ) -> Self {
        Self {
            session,
            hir_krate,
            hir_nodes,
            def_to_hir,
            collected_types: CollectedTypes::new(),
        }
    }

    pub fn collect_items(&mut self) -> CollectedTypes {
        for (def_id, hir_id) in self.def_to_hir {
            let node = self.hir_nodes.get(hir_id).unwrap();
            self.collect_item_def(*def_id, node)
        }

        for (def_id, hir_id) in self.def_to_hir {
            let node = self.hir_nodes.get(hir_id).unwrap();
            self.collect_types(*def_id, node);
        }

        dbg!(&self.collected_types);
        self.collected_types.clone()
    }

    fn collect_item_def(&mut self, def_id: DefId, node: &Node) {
        if let Node::Item(item_kind) = node {
            match &item_kind.node {
                Item::Fn(fn_decl) => {
                    let generic_args = self.lower_generic_params(&fn_decl.sig.node.generics);

                    self.collected_types.generics_of.insert(
                        def_id,
                        Generics {
                            parent: None,
                            params: fn_decl
                                .sig
                                .node
                                .generics
                                .iter()
                                .map(|param| param.node.def_id)
                                .collect(),
                        },
                    );
                    self.collected_types
                        .type_of
                        .insert(def_id, ty::Ty::Fn(def_id, generic_args));
                }
                Item::Struct(struct_decl) => {
                    let generic_args = self.lower_generic_params(&struct_decl.generics);

                    self.collected_types.generics_of.insert(
                        def_id,
                        Generics {
                            parent: None,
                            params: struct_decl.generics.iter().map(|param| param.node.def_id).collect(),
                        },
                    );
                    self.collected_types
                        .type_of
                        .insert(def_id, ty::Ty::Struct(def_id, generic_args));
                }
                Item::Enum(enum_decl) => {
                    let generic_args = self.lower_generic_params(&enum_decl.generics);

                    self.collected_types.generics_of.insert(
                        def_id,
                        Generics {
                            parent: None,
                            params: enum_decl.generics.iter().map(|param| param.node.def_id).collect(),
                        },
                    );
                    self.collected_types
                        .type_of
                        .insert(def_id, ty::Ty::Enum(def_id, generic_args));
                }
                _ => {}
            }
        }
    }

    fn collect_types(&mut self, def_id: DefId, node: &Node) {}

    // fn lower_ty(&self, hir_ty: &HirNode<hir::Ty>) -> ty::Ty {
    //     match &hir_ty.node {
    //         hir::Ty::Path(path) => match &path.node {
    //             Path::Resolved { res, segments } => match res {
    //                 Res::Local(_) => todo!(),
    //                 Res::Def(def_id, def_kind) => match def_kind {
    //                     DefKind::Struct
    //                     | DefKind::Enum
    //                     | DefKind::TypeAlias
    //                     | DefKind::Function
    //                     | DefKind::ExternFn
    //                     | DefKind::AssocFn => self.type_of(*def_id)
    //                     DefKind::TypeParam => ty::Ty::GenericParam(def_id.clone()),
    //                     DefKind::StructField
    //                     | DefKind::EnumVariant
    //                     | DefKind::Trait
    //                     | DefKind::Mod
    //                     | DefKind::Impl
    //                     | DefKind::Use
    //                     | DefKind::Const
    //                     | DefKind::AssocTypeAlias => unreachable!(),
    //                 },
    //                 Res::PrimTy(prim_ty) => match prim_ty {
    //                     PrimTy::I32 => ty::Ty::I32,
    //                     PrimTy::U32 => ty::Ty::U32,
    //                     PrimTy::F64 => ty::Ty::F64,
    //                     PrimTy::Bool => ty::Ty::Bool,
    //                     PrimTy::Str => ty::Ty::Str,
    //                 },
    //                 Res::SelfTy(_) => todo!(),
    //                 Res::Err => ty::Ty::Err,
    //             },
    //             Path::Unresolved {
    //                 res,
    //                 resolved_segments,
    //                 unresolved_segments,
    //             } => todo!(),
    //         },
    //         hir::Ty::Array(elem_ty, _) => {
    //             // TODO: Handle const expressions in array types
    //             ty::Ty::Slice(Box::new(self.lower_ty(elem_ty)))
    //         }
    //         hir::Ty::Ptr(ty) => ty::Ty::Ptr(Box::new(self.lower_ty(ty))),
    //         hir::Ty::Fn(params, return_ty) => {
    //             let params = params.iter().map(|p| self.lower_ty(p)).collect();
    //             let return_ty = return_ty
    //                 .as_ref()
    //                 .map_or(ty::Ty::Unit, |return_ty| self.lower_ty(return_ty));
    //             ty::Ty::FnPtr(params, Box::new(return_ty))
    //         }
    //         hir::Ty::Tuple(types) => {
    //             if types.is_empty() {
    //                 ty::Ty::Unit
    //             } else {
    //                 ty::Ty::Tuple(types.iter().map(|ty| self.lower_ty(ty)).collect())
    //             }
    //         }
    //         hir::Ty::Err => ty::Ty::Err,
    //     }
    // }
    fn lower_generic_params(&self, generic_params: &Vec<HirNode<hir::GenericParam>>) -> GenericArgs {
        generic_params
            .iter()
            .map(|arg| match &arg.node.kind {
                GenericParamKind::Const(_) => todo!(),
                GenericParamKind::Type => GenericArg::Type(ty::Ty::GenericParam(arg.node.def_id)),
            })
            .collect()
    }
}
