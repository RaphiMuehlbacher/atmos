use crate::parser::ast::{AstNode, Ident};
use miette::SourceSpan;
use std::collections::HashMap;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct DefId(usize);

#[derive(Clone, PartialEq, Debug, Default)]
pub struct DefinitionMap {
    definitions: HashMap<DefId, Definition>,
    next_def_id: DefId,
}

impl DefinitionMap {
    pub fn increment_def_id(&mut self) -> DefId {
        let current = self.next_def_id;
        self.next_def_id = DefId(current.0 + 1);
        current
    }

    pub fn insert(&mut self, ident: AstNode<Ident>, kind: DefKind) -> DefId {
        let def_id = self.increment_def_id();
        self.definitions.insert(def_id, Definition::new(def_id, ident, kind));
        def_id
    }

    pub fn get(&self, def_id: &DefId) -> Option<&Definition> {
        self.definitions.get(def_id)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Definition {
    def_id: DefId,
    ident: Ident,
    pub span: SourceSpan,
    kind: DefKind,
}

impl Definition {
    pub fn new(def_id: DefId, ident: AstNode<Ident>, kind: DefKind) -> Self {
        Self {
            def_id,
            ident: ident.node,
            span: ident.span,
            kind,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DefKind {
    Struct,
    StructField,
    Enum,
    EnumVariant,
    Trait,
    Impl,
    Function,
    Const,
    Variable,
    Parameter,
    TypeParam,
    TypeAlias,
    BuiltinType,
}
