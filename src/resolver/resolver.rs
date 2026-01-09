use crate::parser::ast::Crate;
use crate::parser::AstId;
use crate::resolver::collect_defs::DefCollector;
use crate::resolver::defs::DefinitionMap;
use crate::resolver::imports::ImportResolver;
use crate::resolver::late::LateResolver;
use crate::resolver::module_builder::{ModuleArena, ModuleBuilder};
use crate::resolver::modules::{ImportId, ModuleId};
use crate::resolver::visitor::walk_crate;
use crate::Session;
use std::collections::HashMap;

pub struct Resolver<'ast> {
    pub session: &'ast Session,
    ast_program: &'ast Crate,

    pub defs: DefinitionMap,

    pub module_arena: ModuleArena,
    pub modules: HashMap<AstId, ModuleId>,
    pub unresolved_imports: Vec<ImportId>,
}

impl<'ast> Resolver<'ast> {
    pub fn new(session: &'ast Session, ast_program: &'ast Crate) -> Self {
        Self {
            session,
            ast_program,
            defs: DefinitionMap::default(),
            module_arena: ModuleArena::new(),
            modules: HashMap::new(),
            unresolved_imports: Vec::new(),
        }
    }

    pub fn resolve(&mut self) -> &DefinitionMap {
        self.collect_definitions(self.ast_program);
        self.build_modules(self.ast_program);
        self.resolve_imports();
        self.resolve_crate(&self.ast_program);

        &self.defs
    }

    fn collect_definitions(&mut self, krate: &Crate) {
        let mut def_collector = DefCollector::new(self);
        walk_crate(&mut def_collector, krate);
    }

    fn build_modules(&mut self, krate: &Crate) {
        let mut module_builder = ModuleBuilder::new(self, self.module_arena.root_id());
        walk_crate(&mut module_builder, krate);
    }

    fn resolve_imports(&mut self) {
        let mut import_resolver = ImportResolver::new(self);
        import_resolver.resolve();
    }

    fn resolve_crate(&mut self, krate: &Crate) {
        let mut late_resolver = LateResolver::new(self, self.module_arena.root_id());
        walk_crate(&mut late_resolver, krate);
    }
}
