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
    Array(Box<Ty>, Const),
    Slice(Box<Ty>),
    Tuple(Vec<Ty>),
    Ptr(Box<Ty>),
    FnPtr(Vec<Ty>, Box<Ty>),
    Fn(DefId, GenericArgs),
    Struct(DefId, GenericArgs),
    Enum(DefId, GenericArgs),
    Never,
    GenericParam(DefId),
    TyVar(TyVarId),
    Err,
}
