use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::{Crate, GenericParamKind, HirId, HirNode, Item, Node};
use crate::resolver::DefId;
use crate::type_checker::ty;
pub(crate) use crate::type_checker::ty::CollectedTypes;
use crate::type_checker::ty::{GenericArg, GenericArgs, Generics};
use crate::Session;
use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::{GenericParamKind, HirId, HirNode, Item, Node, Path};
use crate::error::CompilerError;
use crate::resolver::DefId;
use crate::resolver::defs::DefKind;
use crate::resolver::ribs::{PrimTy, Res};
use crate::type_checker::error::TypeCheckerError;
use crate::type_checker::ty;
use crate::type_checker::ty::{
    CollectedTypes, EnumDef, FnSig, GenericArg, GenericArgs, Generics, StructDef, VariantData,
};
use std::collections::HashMap;

pub struct TypeCollector<'hir> {
    session: &'hir Session,
    hir_krate: &'hir Crate,
    hir_nodes: &'hir HashMap<HirId, Node>,
    def_to_hir: &'hir HashMap<DefId, HirId>,

    collected_types: CollectedTypes,
    collecting: HashMap<DefId, CollectState>,
}

enum CollectState {
    Done,
    InProgress,
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
            collecting: HashMap::new(),
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

    fn collect_types(&mut self, def_id: DefId, node: &Node) {
        if let Node::Item(item_kind) = node {
            match &item_kind.node {
                Item::Fn(fn_decl) => {
                    let params = fn_decl
                        .sig
                        .node
                        .params
                        .iter()
                        .map(|param| self.lower_ty(&param.node.type_annotation))
                        .collect();

                    let return_ty = fn_decl
                        .sig
                        .node
                        .return_ty
                        .as_ref()
                        .map_or(ty::Ty::Unit, |return_ty| self.lower_ty(&return_ty));
                    self.collected_types.fn_sig.insert(def_id, FnSig { params, return_ty });
                }
                Item::Struct(struct_decl) => {
                    let variant = self.lower_variant(def_id, &struct_decl.data);

                    self.collected_types
                        .structs
                        .insert(def_id, StructDef { def_id, variant });
                }
                Item::Enum(enum_decl) => {
                    let variants = enum_decl
                        .variants
                        .iter()
                        .map(|variant| self.lower_variant(variant.node.def_id, &variant.node.data))
                        .collect();

                    self.collected_types.enums.insert(def_id, EnumDef { def_id, variants });
                }
                Item::Trait(trait_decl) => {}
                Item::Mod(mod_decl) => {}
                Item::Impl(impl_decl) => {}
                Item::ExternFn(extern_fn_decl) => {}
                Item::Const(const_decl) => {}
                Item::TyAlias(ty_alias) => {
                    self.collecting.insert(ty_alias.def_id, CollectState::InProgress);
                    self.collected_types.type_of.insert(def_id, self.lower_ty(&ty_alias.ty));
                    self.collecting.insert(ty_alias.def_id, CollectState::Done);
                }
            }
        }
    }

    fn lower_variant(&mut self, def_id: DefId, variant_data: &HirNode<hir::VariantData>) -> ty::VariantData {
        let fields = match &variant_data.node {
            hir::VariantData::Unit => vec![],
            hir::VariantData::Struct { fields } | hir::VariantData::Tuple { fields } => fields
                .iter()
                .map(|field| {
                    let field_ty = self.lower_ty(&field.node.ty);
                    self.collected_types.type_of.insert(field.node.def_id, field_ty);
                    field.node.def_id
                })
                .collect(),
        };

        VariantData { def_id, fields }
    }

    fn lower_ty(&self, hir_ty: &HirNode<hir::Ty>) -> ty::Ty {
        match &hir_ty.node {
            hir::Ty::Path(path) => match &path.node {
                Path::Resolved { res, segments } => match res {
                    Res::Local(_) => todo!(),
                    Res::Def(def_id, def_kind) => match def_kind {
                        DefKind::Struct | DefKind::Enum | DefKind::Function | DefKind::ExternFn | DefKind::AssocFn => {
                            self.collected_types.type_of.get(def_id).unwrap().clone()
                        }
                        DefKind::TypeAlias => match self.collecting.get(def_id) {
                            Some(CollectState::Done) => self.collected_types.type_of.get(def_id).unwrap().clone(),
                            Some(CollectState::InProgress) => {
                                self.session.push_error(CompilerError::TypeCheckerError(
                                    TypeCheckerError::CyclicTypeDefinition {
                                        src: self.session.get_named_source(),
                                        span: hir_ty.span,
                                        name: segments.last().unwrap().node.ident.node.name.clone(),
                                    },
                                ));
                                ty::Ty::Err
                            }
                            None => {
                                let hir_id = self.def_to_hir.get(def_id).unwrap();
                                let node = self.hir_nodes.get(hir_id).unwrap();

                                let Node::Item(item) = node else { panic!() };
                                let Item::TyAlias(ty_alias) = &item.node else { panic!() };
                                self.lower_ty(&ty_alias.ty)
                            }
                        },
                        DefKind::TypeParam => ty::Ty::GenericParam(def_id.clone()),
                        DefKind::StructField
                        | DefKind::EnumVariant
                        | DefKind::Trait
                        | DefKind::Mod
                        | DefKind::Impl
                        | DefKind::Use
                        | DefKind::Const
                        | DefKind::AssocTypeAlias => unreachable!(),
                    },
                    Res::PrimTy(prim_ty) => match prim_ty {
                        PrimTy::I32 => ty::Ty::I32,
                        PrimTy::U32 => ty::Ty::U32,
                        PrimTy::F64 => ty::Ty::F64,
                        PrimTy::Bool => ty::Ty::Bool,
                        PrimTy::Str => ty::Ty::Str,
                    },
                    Res::SelfTy(_) => todo!(),
                    Res::Err => ty::Ty::Err,
                },
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
