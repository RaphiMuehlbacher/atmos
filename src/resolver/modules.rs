use crate::parser::ast::{Ident, Path};
use crate::resolver::defs::DefId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId(usize);

pub struct Module<'a> {
    parent: Option<&'a Module<'a>>,
    items: RefCell<HashMap<String, Binding<'a>>>,
}

pub enum Binding<'a> {
    Item(DefId),
    Module(Module<'a>),
    Import {
        binding: &'a Binding<'a>,
        import: &'a Import<'a>,
    },
}

pub struct Import<'a> {
    path: Path,
    name: Ident,
    parent_module: &'a Module<'a>,
    resolved_binding: Cell<Option<&'a Binding<'a>>>,
}
