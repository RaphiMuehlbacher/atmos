use crate::parser::ast::{Ident, Path};
use crate::resolver::defs::DefId;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ModuleId(pub usize);

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ImportId(pub usize);

#[derive(Debug, Clone)]
pub struct Module {
    pub kind: ModuleKind,
    parent: Option<ModuleId>,
    items: HashMap<Ident, Binding>,
}

impl Module {
    pub fn root() -> Self {
        Self {
            parent: None,
            items: HashMap::new(),
            kind: ModuleKind::Block,
        }
    }
    pub fn new(parent: ModuleId, kind: ModuleKind) -> Self {
        Self {
            parent: Some(parent),
            items: HashMap::new(),
            kind,
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
pub enum ModuleKind {
    Block,
    Def(DefId),
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
