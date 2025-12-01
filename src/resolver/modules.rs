use crate::parser::ast::{Ident, Path};
use crate::resolver::defs::DefId;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ModuleId(pub usize);

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ImportId(usize);

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
}

#[derive(Debug, Clone)]
pub enum Binding {
    Item(DefId),
    Module(ModuleId),
    Import { binding: Box<Binding>, import: ImportId },
}

#[derive(Debug, Clone)]
pub struct Import {
    path: Path,
    name: Ident,
    parent_module: ModuleId,
    resolved_binding: Option<Binding>,
}
