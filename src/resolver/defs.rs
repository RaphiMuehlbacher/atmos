use crate::parser::ast::{AstNode, Ident, Item};
use crate::parser::AstId;
use miette::SourceSpan;
use std::collections::HashMap;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct DefId(usize);

#[derive(Clone, PartialEq, Debug, Default)]
pub struct DefinitionMap {
    definitions: HashMap<DefId, Definition>,
    ast_to_def: HashMap<AstId, DefId>,
    next_def_id: DefId,
}

impl DefinitionMap {
    pub fn increment_def_id(&mut self) -> DefId {
        let current = self.next_def_id;
        self.next_def_id = DefId(current.0 + 1);
        current
    }

    pub fn insert(&mut self, ident: Option<Ident>, kind: DefKind, span: SourceSpan, id: AstId) -> DefId {
        let def_id = self.increment_def_id();
        self.definitions
            .insert(def_id, Definition::new(def_id, ident, kind, span));
        self.ast_to_def.insert(id, def_id);
        def_id
    }

    pub fn insert_with_ident(&mut self, ident: &AstNode<Ident>, kind: DefKind) -> DefId {
        self.insert(Some(ident.node.clone()), kind, ident.span, ident.ast_id)
    }

    pub fn get(&self, def_id: &DefId) -> Option<&Definition> {
        self.definitions.get(def_id)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Definition {
    def_id: DefId,
    ident: Option<Ident>,
    pub span: SourceSpan,
    kind: DefKind,
}

impl Definition {
    pub fn new(def_id: DefId, ident: Option<Ident>, kind: DefKind, span: SourceSpan) -> Self {
        Self {
            def_id,
            ident,
            span,
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
    Mod,
    Impl,
    Function,
    AssocFn,
    ExternFn,
    Use,
    Const,
    TypeParam,
    TypeAlias,
    AssocTypeAlias,
    BuiltinType,
}

impl From<&Item> for DefKind {
    fn from(value: &Item) -> Self {
        match value {
            Item::Fn(_) => DefKind::Function,
            Item::Struct(_) => DefKind::Struct,
            Item::Enum(_) => DefKind::Enum,
            Item::Trait(_) => DefKind::Trait,
            Item::Mod(_) => DefKind::Mod,
            Item::Impl(_) => DefKind::Impl,
            Item::ExternFn(_) => DefKind::ExternFn,
            Item::Const(_) => DefKind::Const,
            Item::Use(_) => DefKind::Use,
            Item::TyAlias(_) => DefKind::TypeAlias,
            Item::Err => panic!(),
        }
    }
}
