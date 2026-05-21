use crate::ast_lowerer::hir;
use crate::ast_lowerer::hir::{Crate, GenericParamKind, HirId, HirNode, Item, Node, Path, VariantData};
use crate::resolver::defs::DefKind;
use crate::resolver::ribs::{PrimTy, Res};
use crate::resolver::DefId;
use crate::type_checker::ty;
use crate::type_checker::ty::{Const, GenericArg, GenericArgs};
use crate::Session;
use std::collections::HashMap;

pub struct TypeCollector<'hir> {
    session: &'hir Session,
    pub items: HashMap<DefId, ty::Ty>,
    hir_krate: &'hir Crate,
    hir_nodes: &'hir HashMap<HirId, Node>,
    def_to_hir: &'hir HashMap<DefId, HirId>,
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
            items: HashMap::new(),
        }
    }

    pub fn collect_items(&mut self) {
        for item in &self.hir_krate.items {
            self.collect_item_def(item);
        }

        for def_id in self.def_to_hir.keys() {
            let ty = self.type_of(*def_id);
            println!("{def_id:?}: {ty:?}");
        }
    }

    fn collect_item_def(&mut self, item: &HirNode<Item>) {
        match &item.node {
            Item::Fn(fn_decl) => {
                self.items.insert(fn_decl.def_id, self.type_of(fn_decl.def_id));
            }
            Item::Struct(struct_decl) => {
                self.items.insert(struct_decl.def_id, self.type_of(struct_decl.def_id));
            }
            Item::Enum(enum_decl) => {
                self.items.insert(enum_decl.def_id, self.type_of(enum_decl.def_id));
            }
            Item::Trait(_) => {}
            Item::Mod(_) => {}
            Item::Impl(_) => {}
            Item::ExternFn(fn_decl) => {
                self.items.insert(fn_decl.def_id, self.type_of(fn_decl.def_id));
            }
            Item::Const(const_item) => {
                self.items.insert(const_item.def_id, self.lower_ty(&const_item.ty));
            }
            Item::TyAlias(ty_alias_decl) => {
                self.items
                    .insert(ty_alias_decl.def_id, self.lower_ty(&ty_alias_decl.ty));
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

    fn lower_generic_params(&self, generic_params: &Vec<HirNode<hir::GenericParam>>) -> GenericArgs {
        generic_params
            .iter()
            .map(|arg| match &arg.node.kind {
                GenericParamKind::Const(_) => todo!(),
                GenericParamKind::Type => GenericArg::Type(ty::Ty::GenericParam(arg.node.def_id)),
            })
            .collect()
    }

    fn lower_variant(&self, variant: &HirNode<VariantData>) -> ty::VariantData {
        match &variant.node {
            VariantData::Unit => ty::VariantData { fields: vec![] },
            VariantData::Struct { fields } | VariantData::Tuple { fields } => {
                let fields = fields.iter().map(|field| field.node.def_id).collect();
                ty::VariantData { fields }
            }
        }
    }

    fn type_of(&self, def_id: DefId) -> ty::Ty {
        let hir_id = self.def_to_hir.get(&def_id).unwrap();
        let node = self.hir_nodes.get(hir_id).unwrap();

        match node {
            Node::Item(item) => match &item.node {
                Item::Fn(fn_decl) => {
                    let generic_args = self.lower_generic_params(&fn_decl.sig.node.generics);
                    ty::Ty::Fn(def_id, generic_args)
                }
                Item::Struct(struct_decl) => {
                    let generic_args = self.lower_generic_params(&struct_decl.generics);
                    let fields = self.lower_variant(&struct_decl.data);

                    ty::Ty::Struct {
                        def_id,
                        generic_args,
                        fields,
                    }
                }
                Item::Enum(enum_decl) => {
                    let generic_args = self.lower_generic_params(&enum_decl.generics);
                    let variants = enum_decl
                        .variants
                        .iter()
                        .map(|variant| ty::EnumVariant {
                            def_id: variant.node.def_id,
                            variant_data: self.lower_variant(&variant.node.data),
                        })
                        .collect();

                    ty::Ty::Enum {
                        def_id,
                        generic_args,
                        variants,
                    }
                }
                Item::Trait(trait_decl) => todo!(),
                Item::Mod(mod_decl) => todo!(),
                Item::Impl(impl_item) => todo!(),
                Item::ExternFn(extern_fn) => {
                    let generic_args = self.lower_generic_params(&extern_fn.sig.node.generics);
                    ty::Ty::Fn(def_id, generic_args)
                }
                Item::Const(const_decl) => todo!(),
                Item::TyAlias(ty_alias) => self.lower_ty(&ty_alias.ty),
            },
            Node::Param(param) => todo!(),
            Node::FnSig(fn_sig) => todo!(),
            Node::GenericParam(generic_param) => todo!(),
            Node::AssociatedItem(assoc_item) => todo!(),
            Node::Variant(variant) => todo!(),
            Node::VariantData(variant_data) => todo!(),
            Node::Field(field) => self.lower_ty(&field.node.ty),
            Node::Ty(ty) => todo!(),
            Node::Path(path) => todo!(),
            Node::PathSegment(path_segment) => todo!(),
            Node::Ident(ident) => todo!(),
            Node::Pattern(pattern) => todo!(),
            Node::PatField(pat_field) => todo!(),
            Node::Expr(expr) => todo!(),
            Node::ExprField(expr_field) => todo!(),
            Node::Stmt(stmt) => todo!(),
            Node::LetStmt(let_stmt) => todo!(),
            Node::Arm(arm) => todo!(),
            Node::Block(block) => todo!(),
            Node::Err => todo!(),
        }
    }

    fn lower_ty(&self, hir_ty: &HirNode<hir::Ty>) -> ty::Ty {
        match &hir_ty.node {
            hir::Ty::Path(path) => match &path.node {
                Path::Resolved { res, segments } => match res {
                    Res::Local(_) => todo!(),
                    Res::Def(def_id, def_kind) => match def_kind {
                        DefKind::Struct
                        | DefKind::Enum
                        | DefKind::TypeAlias
                        | DefKind::Function
                        | DefKind::ExternFn
                        | DefKind::AssocFn => self.type_of(*def_id),
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
}
