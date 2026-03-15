use crate::ast_lowerer::hir::Expr;
use crate::resolver::DefId;

#[derive(Clone, Copy, PartialEq, Debug, Hash, Eq)]
pub struct TyVarId(u32);

impl TyVarId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn index(self) -> u32 {
        self.0
    }
}

type GenericArgs = Vec<GenericArg>;
pub enum GenericArg {
    Type(Ty),
    Const(Const),
}

pub enum Const {
    Expr(Expr),
}

pub enum Ty {
    Bool,
    I32,
    U32,
    F64,
    Str,
    Array(Box<Ty>, Const),
    Slice(Box<Ty>),
    Tuple(Vec<Ty>),
    Ptr(Box<Ty>),
    Fn(DefId, GenericArgs),
    Struct(DefId, GenericArgs),
    Enum(DefId, GenericArgs),
    GenericParam(DefId),
    TyVar(TyVarId),
}
