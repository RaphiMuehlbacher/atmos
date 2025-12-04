use crate::parser::ast::{Ident, Path};
use crate::resolver::defs::DefId;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ModuleId(pub usize);

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ImportId(pub usize);

#[derive(Debug, Clone, Default)]
pub struct Module {
    parent: Option<ModuleId>,
    items: HashMap<Ident, Binding>,
}

impl Module {
    pub fn new(parent: ModuleId) -> Self {
        Self {
            parent: Some(parent),
            items: HashMap::new(),
        }
    }

    pub fn define(&mut self, ident: Ident, binding: Binding) {
        self.items.insert(ident, binding);
    }

    pub fn get(&self, ident: &Ident) -> Option<&Binding> {
        self.items.get(ident)
    }

    pub fn parent(&self) -> Option<ModuleId> {
        self.parent
    }
}

#[derive(Debug, Clone)]
pub enum Binding {
    Item(DefId),
    Module(ModuleId),
    Import(ImportId),
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: Path,
    pub parent_module: ModuleId,
    pub resolved_binding: Option<Binding>,
}

impl Import {
    pub fn new(path: Path, parent_module: ModuleId) -> Self {
        Self {
            path,
            parent_module,
            resolved_binding: None,
        }
    }
}
