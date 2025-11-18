use crate::parser::AstId;
use crate::resolver::defs::DefId;
use std::collections::HashMap;

#[derive(Default)]
pub struct ResolutionMap {
    map: HashMap<AstId, DefId>,
}

impl ResolutionMap {
    pub fn insert(&mut self, ast_id: AstId, def_id: DefId) {
        self.map.insert(ast_id, def_id);
    }

    pub fn get(&self, ast_id: AstId) -> Option<DefId> {
        self.map.get(&ast_id).cloned()
    }
}
