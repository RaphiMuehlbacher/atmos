use crate::parser::ast::Ident;
use crate::parser::AstId;
use crate::resolver::defs::DefId;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PrimTy {
    I32,
    U32,
    F64,
    Bool,
    Str,
}

impl PrimTy {
    pub fn from_name(name: &str) -> Option<PrimTy> {
        match name {
            "i32" => Some(PrimTy::I32),
            "u32" => Some(PrimTy::U32),
            "f64" => Some(PrimTy::F64),
            "bool" => Some(PrimTy::Bool),
            "str" => Some(PrimTy::Str),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SelfTyInfo {
    /// The DefId of the type that Self refers to (struct, enum, or type alias in impl/trait)
    pub self_ty_def: Option<DefId>,
    /// If inside a trait impl, the DefId of the trait being implemented
    pub trait_def: Option<DefId>,
    /// The DefId of the impl or trait block itself
    pub impl_or_trait_def: DefId,
}

pub enum Res {
    Local(AstId),
    Def(DefId),
    PrimTy(PrimTy),
    SelfTy(SelfTyInfo),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Rib {
    symbols: HashMap<Ident, AstId>,
    kind: RibKind,
}

impl Rib {
    pub fn new(kind: RibKind) -> Self {
        Self {
            symbols: HashMap::new(),
            kind,
        }
    }
    pub fn local() -> Self {
        Self::new(RibKind::Local)
    }
    pub fn item() -> Self {
        Self::new(RibKind::Item)
    }
    pub fn insert(&mut self, name: Ident, def_id: AstId) {
        self.symbols.insert(name, def_id);
    }
    pub fn get(&self, name: &Ident) -> Option<AstId> {
        self.symbols.get(&name).copied()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RibKind {
    Local,
    Item,
}
