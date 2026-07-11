use serde::{Deserialize, Serialize};

use super::ast::AstId;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DefId(pub usize);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Res {
    Local(AstId),
    Def(DefId, DefKind),
    PrimTy(PrimTy),
    SelfTy(SelfTyInfo),
    Err,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefKind {
    Struct,
    StructField,
    Enum,
    EnumVariant,
    Trait,
    Mod,
    Impl,
    Function,
    AssocFn,
    ExternFn,
    Use,
    Const,
    TypeParam,
    TypeAlias,
    AssocTypeAlias,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Definition {
    pub def_id: DefId,
    pub kind: DefKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimTy {
    I32,
    U32,
    F64,
    Bool,
    Str,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelfTyInfo {
    pub self_ty_def: Option<DefId>,
    pub trait_def: Option<DefId>,
    pub impl_or_trait_def: DefId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialRes {
    pub base_res: Res,
    pub unresolved_segments: usize,
}
