use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::defs::DefId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedTypes {
    pub type_of: HashMap<DefId, Ty>,
    pub fn_sig: HashMap<DefId, FnSig>,
    pub structs: HashMap<DefId, StructDef>,
    pub enums: HashMap<DefId, EnumDef>,
    pub traits: HashMap<DefId, TraitDef>,
    pub predicates_of: HashMap<DefId, PredicateDef>,
    pub impls_of: HashMap<DefId, Vec<DefId>>,
    pub assoc_items: HashMap<DefId, Vec<AssocItemDef>>,
    pub generics_of: HashMap<DefId, Generics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Ty {
    Unit,
    Bool,
    I32,
    U32,
    F64,
    Str,
    Never,
    Array(Box<Ty>, String),
    Slice(Box<Ty>),
    Tuple(Vec<Ty>),
    Ptr(Box<Ty>),
    FnPtr(Vec<Ty>, Box<Ty>),
    Fn(DefId, Vec<GenericArg>),
    Struct(DefId, Vec<GenericArg>),
    Enum(DefId, Vec<GenericArg>),
    InherentTyAlias {
        adt_def_id: DefId,
        ident: String,
        resolved_args: Vec<String>,
        unresolved_args: Vec<String>,
    },
    GenericParam(usize),
    TyVar(String),
    Err,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum GenericArg {
    Type(Ty),
    Const(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnSig {
    pub params: Vec<Ty>,
    pub return_ty: Ty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub def_id: DefId,
    pub fields: Vec<DefId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    pub def_id: DefId,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub def_id: DefId,
    pub fields: Vec<DefId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDef {
    pub def_id: DefId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateDef {
    pub def_id: DefId,
    pub ty: Ty,
    pub trait_bound: DefId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssocItemDef {
    pub def_id: DefId,
    pub ident: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generics {
    pub parent: Option<DefId>,
    pub params: Vec<GenericParamDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericParamDef {
    pub def_id: DefId,
    pub index: usize,
}

fn convert_generic_arg(internal: &crate::type_checker::ty::GenericArg) -> GenericArg {
    match internal {
        crate::type_checker::ty::GenericArg::Type(ty) => GenericArg::Type(convert_ty(ty)),
        crate::type_checker::ty::GenericArg::Const(c) => GenericArg::Const(format!("{c:?}")),
    }
}

fn convert_ty(internal: &crate::type_checker::ty::Ty) -> Ty {
    use crate::type_checker::ty::Ty as I;
    match internal {
        I::Unit => Ty::Unit,
        I::Bool => Ty::Bool,
        I::I32 => Ty::I32,
        I::U32 => Ty::U32,
        I::F64 => Ty::F64,
        I::Str => Ty::Str,
        I::Never => Ty::Never,
        I::Array(elem, c) => Ty::Array(Box::new(convert_ty(elem)), format!("{c:?}")),
        I::Slice(elem) => Ty::Slice(Box::new(convert_ty(elem))),
        I::Tuple(tys) => Ty::Tuple(tys.iter().map(convert_ty).collect()),
        I::Ptr(ty) => Ty::Ptr(Box::new(convert_ty(ty))),
        I::FnPtr(params, ret) => {
            Ty::FnPtr(params.iter().map(convert_ty).collect(), Box::new(convert_ty(ret)))
        }
        I::Fn(def_id, args) => Ty::Fn(
            DefId(def_id.0),
            args.iter().map(convert_generic_arg).collect(),
        ),
        I::Struct(def_id, args) => Ty::Struct(
            DefId(def_id.0),
            args.iter().map(convert_generic_arg).collect(),
        ),
        I::Enum(def_id, args) => Ty::Enum(
            DefId(def_id.0),
            args.iter().map(convert_generic_arg).collect(),
        ),
        I::InherentTyAlias {
            adt_def_id,
            ident,
            resolved_args,
            unresolved_args,
        } => Ty::InherentTyAlias {
            adt_def_id: DefId(adt_def_id.0),
            ident: ident.name.clone(),
            resolved_args: resolved_args.iter().map(|a| format!("{a:?}")).collect(),
            unresolved_args: unresolved_args.iter().map(|a| format!("{a:?}")).collect(),
        },
        I::GenericParam(index) => Ty::GenericParam(*index),
        I::TyVar(id) => Ty::TyVar(id.index().to_string()),
        I::Err => Ty::Err,
    }
}

fn convert_fn_sig(internal: &crate::type_checker::ty::FnSig) -> FnSig {
    FnSig {
        params: internal.params.iter().map(convert_ty).collect(),
        return_ty: convert_ty(&internal.return_ty),
    }
}

fn convert_struct_def(internal: &crate::type_checker::ty::StructDef) -> StructDef {
    StructDef {
        def_id: DefId(internal.def_id.0),
        fields: internal.fields.iter().map(|f| DefId(f.0)).collect(),
    }
}

fn convert_enum_def(internal: &crate::type_checker::ty::EnumDef) -> EnumDef {
    EnumDef {
        def_id: DefId(internal.def_id.0),
        variants: internal.variants.iter().map(convert_variant).collect(),
    }
}

fn convert_variant(internal: &crate::type_checker::ty::Variant) -> Variant {
    Variant {
        def_id: DefId(internal.def_id.0),
        fields: internal.fields.iter().map(|f| DefId(f.0)).collect(),
    }
}

fn convert_trait_def(internal: &crate::type_checker::ty::TraitDef) -> TraitDef {
    TraitDef {
        def_id: DefId(internal.def_id.0),
    }
}

fn convert_predicate_def(internal: &crate::type_checker::ty::PredicateDef) -> PredicateDef {
    PredicateDef {
        def_id: DefId(internal.def_id.0),
        ty: convert_ty(&internal.ty),
        trait_bound: DefId(internal.trait_bound.0),
    }
}

fn convert_assoc_item_def(internal: &crate::type_checker::ty::AssocItemDef) -> AssocItemDef {
    AssocItemDef {
        def_id: DefId(internal.def_id.0),
        ident: internal.ident.name.clone(),
    }
}

fn convert_generics(internal: &crate::type_checker::ty::Generics) -> Generics {
    Generics {
        parent: internal.parent.map(|p| DefId(p.0)),
        params: internal.params.iter().map(convert_generic_param_def).collect(),
    }
}

fn convert_generic_param_def(internal: &crate::type_checker::ty::GenericParamDef) -> GenericParamDef {
    GenericParamDef {
        def_id: DefId(internal.def_id.0),
        index: internal.index,
    }
}

impl From<crate::type_checker::ty::CollectedTypes> for CollectedTypes {
    fn from(internal: crate::type_checker::ty::CollectedTypes) -> Self {
        CollectedTypes {
            type_of: internal.type_of.iter().map(|(k, v)| (DefId(k.0), convert_ty(v))).collect(),
            fn_sig: internal.fn_sig.iter().map(|(k, v)| (DefId(k.0), convert_fn_sig(v))).collect(),
            structs: internal.structs.iter().map(|(k, v)| (DefId(k.0), convert_struct_def(v))).collect(),
            enums: internal.enums.iter().map(|(k, v)| (DefId(k.0), convert_enum_def(v))).collect(),
            traits: internal.traits.iter().map(|(k, v)| (DefId(k.0), convert_trait_def(v))).collect(),
            predicates_of: internal.predicates_of.iter().map(|(k, v)| (DefId(k.0), convert_predicate_def(v))).collect(),
            impls_of: internal.impls_of.iter().map(|(k, v)| (DefId(k.0), v.iter().map(|d| DefId(d.0)).collect())).collect(),
            assoc_items: internal.assoc_items.iter().map(|(k, v)| (DefId(k.0), v.iter().map(convert_assoc_item_def).collect())).collect(),
            generics_of: internal.generics_of.iter().map(|(k, v)| (DefId(k.0), convert_generics(v))).collect(),
        }
    }
}
