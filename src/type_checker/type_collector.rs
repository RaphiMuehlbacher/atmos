use crate::Session;
use crate::ast_lowerer::hir::{
    self, AssociatedItem, AssociatedItemKind, GenericParamKind, HirId, HirNode, Item, Node, Path,
};
use crate::error::CompilerError;
use crate::resolver::DefId;
use crate::resolver::defs::DefKind;
use crate::resolver::ribs::{PrimTy, Res, SelfTyInfo};
use crate::type_checker::error::TypeCheckerError;
use crate::type_checker::ty::{
    self, AssocItemDef, CollectedTypes, EnumDef, FnSig, GenericArg, GenericArgs, Generics, StructDef, StructField,
    Variant,
};
use std::collections::HashMap;

pub struct TypeCollector<'hir> {
    session: &'hir Session,
    hir_nodes: &'hir HashMap<HirId, Node>,
    def_to_hir: &'hir HashMap<DefId, HirId>,
    parent_map: &'hir HashMap<DefId, DefId>,

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
        hir_nodes: &'hir HashMap<HirId, Node>,
        def_to_hir: &'hir HashMap<DefId, HirId>,
        parent_map: &'hir HashMap<DefId, DefId>,
    ) -> Self {
        Self {
            session,
            hir_nodes,
            def_to_hir,
            parent_map,
            collected_types: CollectedTypes::new(),
            collecting: HashMap::new(),
        }
    }

    pub fn collect_items(&mut self) -> CollectedTypes {
        for (impl_def_id, hir_id) in self.def_to_hir {
            let node = self.hir_nodes.get(hir_id).unwrap();
            let Node::Item(item) = node else { continue };
            let Item::Impl(impl_decl) = &item.node else { continue };

            match impl_decl.self_ty.node {
                hir::Ty::Path(HirNode {
                    node:
                        Path::Resolved {
                            res: Res::Def(def_id, _),
                            ..
                        },
                    ..
                }) => {
                    self.collected_types
                        .impls_of
                        .entry(def_id)
                        .or_default()
                        .push(*impl_def_id);
                }
                _ => todo!("also collect impls for builtin types and emit error for unresolved paths"),
            }
            for assoc_item in &impl_decl.items {
                self.collected_types
                    .assoc_items
                    .entry(*impl_def_id)
                    .or_default()
                    .push(AssocItemDef {
                        ident: assoc_item.node.ident(),
                        def_id: assoc_item.node.def_id,
                    });
            }
        }

        for (def_id, hir_id) in self.def_to_hir {
            let node = self.hir_nodes.get(hir_id).unwrap();
            self.collect_item_def(*def_id, node);
        }

        for (def_id, hir_id) in self.def_to_hir {
            let node = self.hir_nodes.get(hir_id).unwrap();
            self.collect_types(*def_id, node);
        }

        self.collected_types.clone()
    }

    fn collect_item_def(&mut self, def_id: DefId, node: &Node) {
        if let Node::Item(item_kind) = node {
            match &item_kind.node {
                Item::Struct(struct_decl) => {
                    let generic_params = Generics::without_parent(&struct_decl.generics);
                    self.collected_types.generics_of.insert(def_id, generic_params);

                    let generic_args = self.lower_generic_params(&struct_decl.generics);
                    self.collected_types
                        .type_of
                        .insert(def_id, ty::Ty::Struct(def_id, generic_args));
                }
                Item::Enum(enum_decl) => {
                    let generic_params = Generics::without_parent(&enum_decl.generics);
                    self.collected_types.generics_of.insert(def_id, generic_params);

                    let generic_args = self.lower_generic_params(&enum_decl.generics);
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
                    let generic_params = Generics::without_parent(&fn_decl.sig.node.generics);
                    self.collected_types.generics_of.insert(def_id, generic_params);

                    let generic_args = self.lower_generic_params(&fn_decl.sig.node.generics);
                    self.collected_types
                        .type_of
                        .insert(def_id, ty::Ty::Fn(def_id, generic_args));

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
                        .map_or(ty::Ty::Unit, |return_ty| self.lower_ty(return_ty));

                    self.collected_types.fn_sig.insert(def_id, FnSig { params, return_ty });
                }
                Item::Struct(struct_decl) => {
                    let fields = self.collect_fields(&struct_decl.data);

                    self.collected_types
                        .structs
                        .insert(def_id, StructDef { def_id, fields });
                }
                Item::Enum(enum_decl) => {
                    let variants = enum_decl
                        .variants
                        .iter()
                        .map(|variant| Variant {
                            enum_def: def_id,
                            def_id: variant.node.def_id,
                            fields: self.collect_fields(&variant.node.data),
                        })
                        .collect();

                    self.collected_types.enums.insert(def_id, EnumDef { def_id, variants });
                }
                Item::Trait(trait_decl) => {
                    // TODO: insert into collected_types.traits?
                    let generic_params = Generics::for_trait(def_id, &trait_decl.generics);
                    self.collected_types.generics_of.insert(def_id, generic_params);

                    self.collect_assoc_items(&trait_decl.items);
                }
                Item::Impl(impl_decl) => {
                    let generic_params = Generics::without_parent(&impl_decl.generics);
                    self.collected_types.generics_of.insert(def_id, generic_params);

                    let self_ty = self.lower_ty(&impl_decl.self_ty);
                    self.collected_types.type_of.insert(def_id, self_ty);

                    self.collect_assoc_items(&impl_decl.items);
                }
                Item::Const(const_decl) => {
                    let generic_params = Generics::without_parent(&const_decl.generics);
                    self.collected_types.generics_of.insert(def_id, generic_params);

                    self.collected_types
                        .type_of
                        .insert(def_id, self.lower_ty(&const_decl.ty));
                }
                Item::TyAlias(ty_alias) => {
                    let generic_params = Generics::without_parent(&ty_alias.generics);
                    self.collected_types.generics_of.insert(def_id, generic_params);

                    self.collecting.insert(def_id, CollectState::InProgress);
                    self.collected_types.type_of.insert(def_id, self.lower_ty(&ty_alias.ty));
                    self.collecting.insert(def_id, CollectState::Done);
                }
                Item::ExternFn(_) => todo!(),
                Item::Mod(_) => {}
            }
        }
    }

    fn collect_assoc_items(&mut self, assoc_items: &[HirNode<AssociatedItem>]) {
        for assoc in assoc_items {
            let params = match &assoc.node.kind {
                AssociatedItemKind::Fn(sig, _) => &sig.node.generics.as_slice(),
                AssociatedItemKind::Type(ty_alias) => &ty_alias.node.generics.as_slice(),
            };

            let parent_def_id = self.parent_map.get(&assoc.node.def_id).unwrap();
            let parent_generics = self.collected_types.generics_of.get(parent_def_id).unwrap();

            assert_eq!(
                parent_generics.parent, None,
                "We only support associated items nested one level"
            );

            let generic_params = Generics::with_parent(*parent_def_id, params, parent_generics.params.len());
            self.collected_types
                .generics_of
                .insert(assoc.node.def_id, generic_params);

            match &assoc.node.kind {
                AssociatedItemKind::Fn(sig, _) => {
                    let generic_args = self.lower_generic_params(&sig.node.generics);
                    self.collected_types
                        .type_of
                        .insert(assoc.node.def_id, ty::Ty::Fn(assoc.node.def_id, generic_args));

                    let params = sig
                        .node
                        .params
                        .iter()
                        .map(|param| self.lower_ty(&param.node.type_annotation))
                        .collect();

                    let return_ty = sig
                        .node
                        .return_ty
                        .as_ref()
                        .map_or(ty::Ty::Unit, |return_ty| self.lower_ty(return_ty));

                    self.collected_types
                        .fn_sig
                        .insert(assoc.node.def_id, FnSig { params, return_ty });
                }
                AssociatedItemKind::Type(ty_alias) => {
                    if let Some(ty) = &ty_alias.node.ty {
                        self.collecting.insert(ty_alias.node.def_id, CollectState::InProgress);
                        self.collected_types
                            .type_of
                            .insert(ty_alias.node.def_id, self.lower_ty(ty));
                        self.collecting.insert(ty_alias.node.def_id, CollectState::Done);
                    }
                }
            }
        }
    }

    fn lower_generic_params(&self, generic_params: &[HirNode<hir::GenericParam>]) -> GenericArgs {
        generic_params
            .iter()
            .map(|arg| match &arg.node.kind {
                GenericParamKind::Const(_) => todo!(),
                GenericParamKind::Type => {
                    let parent_def_id = self.parent_map.get(&arg.node.def_id).unwrap();
                    let index = self
                        .collected_types
                        .generics_of
                        .get(parent_def_id)
                        .unwrap()
                        .get_index(arg.node.def_id, &self.collected_types.generics_of);

                    GenericArg::Type(ty::Ty::GenericParam(index))
                }
            })
            .collect()
    }

    fn lower_generic_args(&self, generic_args: &[HirNode<hir::GenericArg>]) -> ty::GenericArgs {
        generic_args
            .iter()
            .map(|arg| match &arg.node {
                hir::GenericArg::Type(ty) => GenericArg::Type(self.lower_ty(ty)),
                hir::GenericArg::Const(_) => todo!(),
            })
            .collect()
    }

    fn collect_fields(&mut self, variant: &HirNode<hir::VariantData>) -> Vec<StructField> {
        match &variant.node {
            hir::VariantData::Unit => vec![],
            hir::VariantData::Struct { fields } | hir::VariantData::Tuple { fields } => fields
                .iter()
                .map(|field| {
                    let field_ty = self.lower_ty(&field.node.ty);
                    self.collected_types.type_of.insert(field.node.def_id, field_ty);
                    StructField {
                        ident: field.node.ident.node.clone(),
                        def_id: field.node.def_id,
                    }
                })
                .collect(),
        }
    }

    fn lower_ty(&self, hir_ty: &HirNode<hir::Ty>) -> ty::Ty {
        match &hir_ty.node {
            hir::Ty::Path(path) => match &path.node {
                Path::Resolved { res, segments } => match res {
                    Res::Local(_) => todo!("can this happen?"),
                    Res::Def(def_id, def_kind) => match def_kind {
                        DefKind::Struct => {
                            let last_segment = &segments.last().unwrap();
                            let args = &last_segment.node.args;
                            let generics = self.collected_types.generics_of.get(def_id).unwrap();
                            let args = self.lower_generic_args(args);

                            if generics.params.len() != args.len() {
                                self.session.push_error(CompilerError::TypeCheckerError(
                                    TypeCheckerError::GenericArgArityMismatch {
                                        src: self.session.get_named_source(),
                                        span: last_segment.span,
                                        name: last_segment.node.ident.node.name.clone(),
                                        expected: generics.params.len(),
                                        found: args.len(),
                                    },
                                ));
                            }

                            ty::Ty::Struct(*def_id, args)
                        }
                        DefKind::Enum => {
                            let last_segment = &segments.last().unwrap();
                            let args = &last_segment.node.args;
                            let generics = self.collected_types.generics_of.get(def_id).unwrap();
                            let args = self.lower_generic_args(args);

                            if generics.params.len() != args.len() {
                                self.session.push_error(CompilerError::TypeCheckerError(
                                    TypeCheckerError::GenericArgArityMismatch {
                                        src: self.session.get_named_source(),
                                        span: last_segment.span,
                                        name: last_segment.node.ident.node.name.clone(),
                                        expected: generics.params.len(),
                                        found: args.len(),
                                    },
                                ));
                            }

                            ty::Ty::Enum(*def_id, args)
                        }
                        DefKind::Function | DefKind::ExternFn | DefKind::AssocFn => {
                            // TODO: handle generic args
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
                        DefKind::GenericParam => {
                            let parent_def_id = self.parent_map.get(def_id).unwrap();
                            let index = self
                                .collected_types
                                .generics_of
                                .get(parent_def_id)
                                .unwrap()
                                .get_index(*def_id, &self.collected_types.generics_of);

                            ty::Ty::GenericParam(index)
                        }
                        DefKind::StructField
                        | DefKind::EnumVariant { .. }
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
                    Res::SelfTy(SelfTyInfo {
                        self_ty_def, trait_def, ..
                    }) => {
                        if let Some(def_id) = self_ty_def {
                            self.collected_types.type_of.get(def_id).unwrap().clone()
                        } else if trait_def.is_some() {
                            ty::Ty::GenericParam(0)
                        } else {
                            unreachable!()
                        }
                    }
                    Res::Err => ty::Ty::Err,
                },
                Path::Unresolved {
                    res,
                    resolved_segments,
                    unresolved_segments,
                } => match res {
                    Res::Def(def_id, _) => {
                        assert_eq!(
                            unresolved_segments.len(),
                            1,
                            "currently only 1 associated item should be possible"
                        );
                        let impls = self.collected_types.impls_of.get(def_id).unwrap();
                        let assoc_types = impls
                            .iter()
                            .map(|impl_def| self.collected_types.assoc_items.get(impl_def).unwrap())
                            .flat_map(|assoc_items| assoc_items.iter().map(|assoc| assoc.def_id))
                            .collect();

                        let path = unresolved_segments.first().unwrap().node.clone();
                        ty::Ty::InherentTyAlias {
                            candidates: assoc_types,
                            ident: path.ident.node,
                            resolved_args: resolved_segments
                                .last()
                                .unwrap()
                                .node
                                .args
                                .iter()
                                .map(|arg| arg.node.clone())
                                .collect(),
                            unresolved_args: path.args.iter().map(|arg| arg.node.clone()).collect(),
                        }
                    }
                    Res::SelfTy(self_ty_info) => todo!(),
                    _ => todo!(),
                },
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
