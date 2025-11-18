use crate::resolver::defs::DefId;
use std::collections::HashMap;

pub struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    symbols: HashMap<String, DefId>,
}

impl<'a> Scope<'a> {
    pub fn new(parent: Option<&'a Scope<'a>>) -> Self {
        Self {
            parent,
            symbols: HashMap::new(),
        }
    }

    pub fn child(&'a self) -> Self {
        Self::new(Some(self))
    }

    pub fn insert(&mut self, name: String, def_id: DefId) {
        self.symbols.insert(name, def_id);
    }

    pub fn lookup(&self, name: &str) -> Option<DefId> {
        match self.symbols.get(name) {
            Some(def) => Some(*def),
            None => self.parent?.lookup(name),
        }
    }
}

impl<'a> Default for Scope<'a> {
    fn default() -> Self {
        Self::new(None)
    }
}
