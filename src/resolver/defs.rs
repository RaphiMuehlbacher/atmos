use crate::parser::AstId;
use crate::parser::ast::Item;
use crate::resolver::ribs::Res;
use std::collections::HashMap;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct DefId(usize);

#[derive(Clone, PartialEq, Debug, Default)]
pub struct DefinitionMap {
    pub definitions: HashMap<DefId, Definition>,
    pub resolutions: HashMap<AstId, Res>,
    pub ast_to_def: HashMap<AstId, DefId>,
    pub partial_res: HashMap<AstId, PartialRes>,
    next_def_id: DefId,
}

impl DefinitionMap {
    pub fn increment_def_id(&mut self) -> DefId {
        let current = self.next_def_id;
        self.next_def_id = DefId(current.0 + 1);
        current
    }

    pub fn insert(&mut self, id: AstId, kind: DefKind) -> DefId {
        let def_id = self.increment_def_id();
        self.definitions.insert(def_id, Definition::new(def_id, kind));
        self.ast_to_def.insert(id, def_id);
        def_id
    }

    pub fn insert_ast_id(&mut self, ast_id: AstId, def_id: DefId) {
        self.ast_to_def.insert(ast_id, def_id);
    }

    #[must_use]
    pub fn get_definition(&self, def_id: DefId) -> Option<&Definition> {
        self.definitions.get(&def_id)
    }

    #[must_use]
    pub fn get_def_from_ast(&self, ast_id: AstId) -> Option<&DefId> {
        self.ast_to_def.get(&ast_id)
    }

    pub fn insert_resolution(&mut self, ast_id: AstId, resolution: Res) {
        self.resolutions.insert(ast_id, resolution);
    }

    #[must_use]
    pub fn get_resolution(&self, ast_id: AstId) -> Option<&Res> {
        self.resolutions.get(&ast_id)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Definition {
    pub def_id: DefId,
    pub kind: DefKind,
}

impl Definition {
    #[must_use]
    pub fn new(def_id: DefId, kind: DefKind) -> Self {
        Self { def_id, kind }
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
}

impl From<&Item> for DefKind {
    fn from(value: &Item) -> Self {
        match value {
            Item::Fn(_) => Self::Function,
            Item::Struct(_) => Self::Struct,
            Item::Enum(_) => Self::Enum,
            Item::Trait(_) => Self::Trait,
            Item::Mod(_) => Self::Mod,
            Item::Impl(_) => Self::Impl,
            Item::ExternFn(_) => Self::ExternFn,
            Item::Const(_) => Self::Const,
            Item::Use(_) => Self::Use,
            Item::TyAlias(_) => Self::TypeAlias,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct PartialRes {
    base_res: Res,
    unresolved_segments: usize,
}

impl PartialRes {
    #[must_use]
    pub fn new(base_res: Res, unresolved_segments: usize) -> Self {
        Self {
            base_res,
            unresolved_segments,
        }
    }

    #[must_use]
    pub fn base_res(&self) -> Res {
        self.base_res.clone()
    }

    #[must_use]
    pub fn unresolved_segments(&self) -> usize {
        self.unresolved_segments
    }
}
