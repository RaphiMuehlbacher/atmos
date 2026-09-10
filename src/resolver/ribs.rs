use crate::ast_lowerer::hir::HirId;
use crate::parser::AstId;
use crate::parser::ast::Ident;
use crate::resolver::DefId;
use crate::resolver::defs::DefKind;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrimTy {
    I32,
    U32,
    F64,
    Bool,
    Str,
}

impl PrimTy {
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "i32" => Some(Self::I32),
            "u32" => Some(Self::U32),
            "f64" => Some(Self::F64),
            "bool" => Some(Self::Bool),
            "str" => Some(Self::Str),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum SelfTyKind {
    /// `Self` inside a struct/enum definition, where `alias_to` is the type being defined.
    AdtDef { alias_to: DefId },
    /// `Self` inside a trait definition where `trait_def` is the trait itself.
    TraitDef { trait_def: DefId },
    /// `Self` inside an impl_block, where `impl_block` is the impl/trait block itself.
    Impl { impl_block: DefId },
}

#[derive(Clone, PartialEq, Debug)]
pub enum Res<Id = HirId> {
    Local(Id),
    Def(DefId, DefKind),
    PrimTy(PrimTy),
    SelfTy(SelfTyKind),
    Err,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Rib {
    symbols: HashMap<Ident, Res<AstId>>,
    kind: RibKind,
}

impl Rib {
    #[must_use]
    pub fn new(kind: RibKind) -> Self {
        Self {
            symbols: HashMap::new(),
            kind,
        }
    }

    #[must_use]
    pub fn local() -> Self {
        Self::new(RibKind::Local)
    }

    #[must_use]
    pub fn item() -> Self {
        Self::new(RibKind::Item)
    }

    pub fn insert(&mut self, name: Ident, res: Res<AstId>) {
        self.symbols.insert(name, res);
    }

    #[must_use]
    pub fn get(&self, name: &Ident) -> Option<Res<AstId>> {
        self.symbols.get(name).cloned()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RibKind {
    Local,
    Item,
}
