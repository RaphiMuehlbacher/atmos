use crate::ast_lowerer::hir::Expr;
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
    GenericParam(DefId),
    TyVar(TyVarId),
    Err,
}

#[derive(Clone, Debug)]
pub struct VariantData {
    pub def_id: DefId,
    pub fields: Vec<DefId>,
}

#[derive(Clone, Debug)]
pub struct EnumVariant {
    pub def_id: DefId,
    pub variant_data: VariantData,
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub params: Vec<Ty>,
    pub return_ty: Ty,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub def_id: DefId,
    pub variant: VariantData,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub def_id: DefId,
    pub variants: Vec<VariantData>,
}

#[derive(Debug, Clone)]
pub struct Generics {
    pub parent: Option<DefId>,
    pub params: Vec<DefId>,
}

#[derive(Debug, Clone, Default)]
pub struct CollectedTypes {
    pub type_of: HashMap<DefId, Ty>,
    pub fn_sig: HashMap<DefId, FnSig>,
    pub structs: HashMap<DefId, StructDef>,
    pub enums: HashMap<DefId, EnumDef>,
    pub generics_of: HashMap<DefId, Generics>,
}

impl CollectedTypes {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
