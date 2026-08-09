use crate::ast_lowerer::hir::{self, Expr, GenericParam, HirNode};
use crate::parser::ast::Ident;
use crate::resolver::DefId;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug, Hash, Eq)]
pub struct TyVarId(u32);

impl TyVarId {
    #[must_use]
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn index(self) -> u32 {
        self.0
    }
}

pub type GenericArgs = Vec<GenericArg>;

#[derive(Clone, Debug)]
pub enum GenericArg {
    Type(Ty),
    Const(Const),
}

#[derive(Clone, Debug)]
pub enum Const {
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub enum Ty {
    Unit,
    Bool,
    I32,
    U32,
    F64,
    Str,
    Never,
    Array(Box<Ty>, Const),
    Slice(Box<Ty>),
    Tuple(Vec<Ty>),
    Ptr(Box<Ty>),
    FnPtr(Vec<Ty>, Box<Ty>),
    Fn(DefId, GenericArgs),
    Struct(DefId, GenericArgs),
    Enum(DefId, GenericArgs),
    /// `DefId` of Adt
    InherentTyAlias {
        candidates: Vec<DefId>,
        ident: Ident,
        resolved_args: Vec<hir::GenericArg>,
        unresolved_args: Vec<hir::GenericArg>,
    },
    GenericParam(usize),
    Infer(InferTy),
    Err,
}

#[derive(Clone, Debug)]
pub enum InferTy {
    TyVar(TyVarId),
    IntVar(TyVarId),
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub def_id: DefId,
    pub fields: Vec<DefId>,
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub params: Vec<Ty>,
    pub return_ty: Ty,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub def_id: DefId,
    pub fields: Vec<DefId>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub def_id: DefId,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub def_id: DefId,
}

#[derive(Debug, Clone)]
pub struct PredicateDef {
    pub def_id: DefId,
    pub ty: Ty,
    pub trait_bound: DefId,
}

#[derive(Debug, Clone)]
pub struct AssocItemDef {
    pub def_id: DefId,
    pub ident: Ident,
    // TODO: maybe needed or changed
    // pub parent: AssocParent,
}

// #[derive(Debug, Clone)]
// pub enum AssocParent {
//     Trait,
//     Impl,
//     TraitImpl,
// }

#[derive(Debug, Clone)]
pub struct Generics {
    pub parent: Option<DefId>,
    pub params: Vec<GenericParamDef>,
}

#[derive(Debug, Clone)]
pub struct GenericParamDef {
    pub def_id: DefId,
    pub index: usize,
    pub kind: GenericParamKind,
}

#[derive(Debug, Clone)]
pub enum GenericParamKind {
    Type,
    Const,
}

impl From<hir::GenericParamKind> for GenericParamKind {
    fn from(kind: hir::GenericParamKind) -> Self {
        match kind {
            hir::GenericParamKind::Const(_) => Self::Const,
            hir::GenericParamKind::Type => Self::Type,
        }
    }
}

impl Generics {
    pub fn new(parent: Option<DefId>, params: &[HirNode<GenericParam>], index_start: usize) -> Self {
        let params = params
            .iter()
            .enumerate()
            .map(|(i, param)| GenericParamDef {
                def_id: param.node.def_id,
                index: index_start + i,
                kind: param.node.kind.clone().into(),
            })
            .collect();

        Self { parent, params }
    }

    pub fn with_parent(parent: DefId, params: &[HirNode<GenericParam>], index_start: usize) -> Self {
        Self::new(Some(parent), params, index_start)
    }

    pub fn without_parent(params: &[HirNode<GenericParam>]) -> Self {
        Self::new(None, params, 0)
    }

    pub fn for_trait(trait_def_id: DefId, params: &[HirNode<GenericParam>]) -> Self {
        let mut generics = Self::new(None, params, 1);
        generics.params.insert(
            0,
            GenericParamDef {
                def_id: trait_def_id,
                index: 0,
                kind: GenericParamKind::Type,
            },
        );
        generics
    }

    pub fn get_index(&self, def_id: DefId, generics_of: &HashMap<DefId, Generics>) -> usize {
        let param = self.params.iter().find(|param| param.def_id == def_id);
        match param {
            Some(param) => param.index,
            None if let Some(parent_def_id) = self.parent => {
                Generics::get_index(generics_of.get(&parent_def_id).unwrap(), def_id, &generics_of)
            }
            None => panic!(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CollectedTypes {
    pub type_of: HashMap<DefId, Ty>,
    pub fn_sig: HashMap<DefId, FnSig>,
    pub structs: HashMap<DefId, StructDef>,
    pub enums: HashMap<DefId, EnumDef>,
    pub traits: HashMap<DefId, TraitDef>,
    /// `DefId` of Generic Param
    pub predicates_of: HashMap<DefId, PredicateDef>,
    pub impls_of: HashMap<DefId, Vec<DefId>>,
    /// `DefId` of Impl or Trait
    pub assoc_items: HashMap<DefId, Vec<AssocItemDef>>,
    pub generics_of: HashMap<DefId, Generics>,
}

impl CollectedTypes {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
