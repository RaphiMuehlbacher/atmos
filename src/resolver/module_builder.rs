use crate::parser::ast::{AstNode, Ident, Item};
use crate::resolver::modules::{Binding, Module, ModuleId};
use crate::resolver::visitor;
use crate::Resolver;

#[derive(Debug, Clone)]
pub struct ModuleArena {
    modules: Vec<Module>,
    root_id: ModuleId,
}

impl ModuleArena {
    pub fn new() -> Self {
        let mut modules = Vec::new();
        let root_module = Module::default();
        modules.push(root_module);
        let root_id = ModuleId(0);

        Self { modules, root_id }
    }

    pub fn root_id(&self) -> ModuleId {
        self.root_id
    }

    pub fn get(&self, id: ModuleId) -> &Module {
        &self.modules[id.0]
    }

    pub fn get_mut(&mut self, id: ModuleId) -> &mut Module {
        &mut self.modules[id.0]
    }

    pub fn add_module(&mut self, parent: ModuleId) -> ModuleId {
        let id = ModuleId(self.modules.len());
        self.modules.push(Module::new(parent));
        id
    }

    pub fn define(&mut self, module_id: ModuleId, ident: Ident, binding: Binding) {
        self.get_mut(module_id).define(ident, binding);
    }
}

pub struct ModuleBuilder<'a, 'r> {
    r: &'a mut Resolver<'r>,
    parent: ModuleId,
}

impl<'a, 'r> ModuleBuilder<'a, 'r> {
    pub fn new(resolver: &'a mut Resolver<'r>, parent: ModuleId) -> Self {
        Self { r: resolver, parent }
    }
}

impl<'a, 'r> visitor::Visitor for ModuleBuilder<'a, 'r> {
    fn visit_item(&mut self, item: &AstNode<Item>) {
        let parent = self.parent;

        match &item.node {
            Item::Mod(module_decl) => {
                let module = self.r.module_arena.add_module(self.parent);
                self.r
                    .module_arena
                    .define(parent, module_decl.ident.node.clone(), Binding::Module(module));

                self.r.modules.insert(item.ast_id, module);
                self.parent = module;
            }
            Item::Fn(fn_decl) => {
                let def_id = self.r.defs.get_def_from_ast(item.ast_id).unwrap();
                self.r
                    .module_arena
                    .define(parent, fn_decl.sig.node.ident.node.clone(), Binding::Item(*def_id));
            }
            Item::Struct(struct_decl) => {
                let def_id = self.r.defs.get_def_from_ast(item.ast_id).unwrap();
                self.r
                    .module_arena
                    .define(parent, struct_decl.ident.node.clone(), Binding::Item(*def_id));
            }
            Item::Enum(enum_decl) => {
                let module = self.r.module_arena.add_module(self.parent);
                let def_id = self.r.defs.get_def_from_ast(item.ast_id).unwrap();
                self.r
                    .module_arena
                    .define(parent, enum_decl.ident.node.clone(), Binding::Item(*def_id));
                self.parent = module;
            }
            Item::Trait(trait_decl) => {
                let module = self.r.module_arena.add_module(self.parent);
                let def_id = self.r.defs.get_def_from_ast(item.ast_id).unwrap();
                self.r
                    .module_arena
                    .define(parent, trait_decl.ident.node.clone(), Binding::Item(*def_id));
                self.parent = module;
            }
            Item::Const(const_decl) => {
                let def_id = self.r.defs.get_def_from_ast(item.ast_id).unwrap();
                self.r
                    .module_arena
                    .define(parent, const_decl.ident.node.clone(), Binding::Item(*def_id));
            }
            Item::ExternFn(extern_fn_decl) => {
                let def_id = self.r.defs.get_def_from_ast(item.ast_id).unwrap();
                self.r.module_arena.define(
                    parent,
                    extern_fn_decl.sig.node.ident.node.clone(),
                    Binding::Item(*def_id),
                );
            }
            Item::Use(use_decl) => {}
            Item::TyAlias(ty_alias_decl) => {
                let def_id = self.r.defs.get_def_from_ast(item.ast_id).unwrap();
                self.r
                    .module_arena
                    .define(parent, ty_alias_decl.ident.node.clone(), Binding::Item(*def_id));
            }
            Item::Impl(_) | Item::Err => {}
        }
        visitor::walk_item(self, item);
        self.parent = parent;
    }
}
