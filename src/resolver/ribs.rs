use crate::parser::ast::Ident;
use crate::parser::AstId;
use crate::resolver::defs::DefId;
use std::collections::HashMap;

pub enum Res {
    Local(AstId),
    Def(DefId),
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
