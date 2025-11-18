use crate::parser::ast::{AstNode, Crate, Item};
use crate::resolver::defs::DefinitionMap;
use crate::resolver::resolutions::ResolutionMap;
use crate::resolver::scopes::Scope;

struct Resolver<'ast> {
    ast_program: &'ast Crate,
    current_scope: Scope<'ast>,
    resolutions: ResolutionMap,
    definitions: DefinitionMap,
}

impl<'ast> Resolver<'ast> {
    fn new(ast_program: &'ast Crate) -> Self {
        Resolver {
            ast_program,
            current_scope: Scope::default(),
            resolutions: ResolutionMap::default(),
            definitions: DefinitionMap::default(),
        }
    }

    pub fn resolve(&mut self) {
        for item in &self.ast_program.items {
            self.resolve_item(item);
        }
    }

    fn resolve_item(&mut self, item: &AstNode<Item>) {
        self.declare_item(item);
    }

    fn declare_item(&mut self, item: &AstNode<Item>) {
        match &item.node {
            Item::Fn(fn_decl) => {}
            Item::Struct(struct_decl) => {}
            Item::Enum(enum_decl) => {}
            Item::Trait(trait_decl) => {}
            Item::Impl(impl_decl) => {}
            Item::ExternFn(extern_fn_decl) => {}
            Item::Const(const_decl) => {}
            Item::Use(use_item) => {}
            Item::TyAlias(ty_alias_decl) => {}
            Item::Err => {}
        }
    }
}
