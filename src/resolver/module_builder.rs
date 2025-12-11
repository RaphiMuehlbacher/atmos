use crate::parser::ast::{AssociatedItem, AstNode, BlockExpr, EnumVariant, GenericParam, Ident, Item};
use crate::resolver::modules::{Binding, Import, ImportId, Module, ModuleId, ModuleKind};
use crate::resolver::visitor;
use crate::Resolver;

#[derive(Debug, Clone)]
pub struct ModuleArena {
    modules: Vec<Module>,
    imports: Vec<Import>,
    root_id: ModuleId,
}

impl ModuleArena {
    pub fn new() -> Self {
        let mut modules = Vec::new();
        let root_module = Module::root();
        modules.push(root_module);
        let root_id = ModuleId(0);

        Self {
            modules,
            root_id,
            imports: Vec::new(),
        }
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

    pub fn add_module(&mut self, parent: ModuleId, kind: ModuleKind) -> ModuleId {
        let id = ModuleId(self.modules.len());
        self.modules.push(Module::new(parent, kind));
        id
    }

    pub fn define(&mut self, module_id: ModuleId, ident: Ident, binding: Binding) {
        self.get_mut(module_id).define(ident, binding);
    }

    pub fn add_import(&mut self, import: Import) -> ImportId {
        let id = ImportId(self.imports.len());
        self.imports.push(import);
        id
    }

    pub fn get_import(&self, id: ImportId) -> &Import {
        &self.imports[id.0]
    }

    pub fn get_import_mut(&mut self, id: ImportId) -> &mut Import {
        &mut self.imports[id.0]
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
        let def_id = self.r.defs.get_def_from_ast(item.ast_id).unwrap();

        match &item.node {
            Item::Mod(module_decl) => {
                let module = self.r.module_arena.add_module(self.parent, ModuleKind::Def(*def_id));
                self.r
                    .module_arena
                    .define(parent, module_decl.ident.node.clone(), Binding::Module(module));

                self.r.modules.insert(item.ast_id, module);
                self.parent = module;
            }
            Item::Fn(fn_decl) => {
                self.r
                    .module_arena
                    .define(parent, fn_decl.sig.node.ident.node.clone(), Binding::Item(*def_id));
            }
            Item::Struct(struct_decl) => {
                self.r
                    .module_arena
                    .define(parent, struct_decl.ident.node.clone(), Binding::Item(*def_id));
            }
            Item::Enum(enum_decl) => {
                let module = self.r.module_arena.add_module(self.parent, ModuleKind::Def(*def_id));
                self.r
                    .module_arena
                    .define(parent, enum_decl.ident.node.clone(), Binding::Item(*def_id));
                self.r.modules.insert(item.ast_id, module);
                self.parent = module;
            }
            Item::Trait(trait_decl) => {
                let module = self.r.module_arena.add_module(self.parent, ModuleKind::Def(*def_id));
                self.r
                    .module_arena
                    .define(parent, trait_decl.ident.node.clone(), Binding::Item(*def_id));
                self.r.modules.insert(item.ast_id, module);
                self.parent = module;
            }
            Item::Const(const_decl) => {
                self.r
                    .module_arena
                    .define(parent, const_decl.ident.node.clone(), Binding::Item(*def_id));
            }
            Item::ExternFn(extern_fn_decl) => {
                self.r.module_arena.define(
                    parent,
                    extern_fn_decl.sig.node.ident.node.clone(),
                    Binding::Item(*def_id),
                );
            }
            Item::Use(use_decl) => {
                let path = &use_decl.path.node;
                let import = Import::new(path.clone(), parent);
                let import_id = self.r.module_arena.add_import(import);

                self.r.unresolved_imports.push(import_id);
            }
            Item::TyAlias(ty_alias_decl) => {
                self.r
                    .module_arena
                    .define(parent, ty_alias_decl.ident.node.clone(), Binding::Item(*def_id));
            }
            Item::Impl(_) | Item::Err => {}
        }
        visitor::walk_item(self, item);
        self.parent = parent;
    }

    fn visit_generic_param(&mut self, generic_param: &AstNode<GenericParam>) {
        let def_id = self.r.defs.get_def_from_ast(generic_param.ast_id).unwrap();
        self.r.module_arena.define(
            self.parent,
            generic_param.node.ident.node.clone(),
            Binding::Item(*def_id),
        );
    }

    fn visit_assoc_item(&mut self, assoc_item: &AstNode<AssociatedItem>) {
        let ident = match &assoc_item.node {
            AssociatedItem::Fn(sig, _) => sig.node.ident.node.clone(),
            AssociatedItem::Type(ty_alias) => ty_alias.node.ident.node.clone(),
        };

        let def_id = self.r.defs.get_def_from_ast(assoc_item.ast_id).unwrap();
        self.r.module_arena.define(self.parent, ident, Binding::Item(*def_id))
    }

    fn visit_enum_variant(&mut self, enum_variant: &AstNode<EnumVariant>) {
        let def_id = self.r.defs.get_def_from_ast(enum_variant.ast_id).unwrap();
        self.r.module_arena.define(
            self.parent,
            enum_variant.node.ident.node.clone(),
            Binding::Item(*def_id),
        );
    }

    fn visit_block(&mut self, block: &AstNode<BlockExpr>) {
        let module = self.r.module_arena.add_module(self.parent, ModuleKind::Block);
        self.r.modules.insert(block.ast_id, module);
    }
}
