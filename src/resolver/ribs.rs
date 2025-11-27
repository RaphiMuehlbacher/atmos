use crate::resolver::defs::DefId;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Debug)]
pub struct Rib {
    symbols: HashMap<String, DefId>,
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
    pub fn insert(&mut self, name: String, def_id: DefId) {
        self.symbols.insert(name, def_id);
    }
    pub fn get(&self, name: &str) -> Option<DefId> {
        self.symbols.get(name).copied()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RibKind {
    Local,
    Item,
}
