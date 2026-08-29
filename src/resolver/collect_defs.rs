use crate::Resolver;
use crate::parser::ast::{AssociatedItem, AstNode, EnumVariant, FieldDef, GenericParam, Item};
use crate::resolver::defs::DefKind;
use crate::resolver::{DefId, visitor};

pub struct DefCollector<'a, 'r> {
    r: &'a mut Resolver<'r>,
    def_stack: Vec<DefId>,
}

impl<'a, 'r> DefCollector<'a, 'r> {
    pub fn new(r: &'a mut Resolver<'r>) -> Self {
        Self { r, def_stack: vec![] }
    }
}

impl visitor::Visitor for DefCollector<'_, '_> {
    fn visit_item(&mut self, item: &AstNode<Item>) {
        let def_kind = DefKind::from(&item.node);

        let def_id = self.r.defs.insert(item.ast_id, def_kind, self.def_stack.last());

        self.def_stack.push(def_id);
        visitor::walk_item(self, item);
        self.def_stack.pop();
    }

    fn visit_generic_param(&mut self, generic_param: &AstNode<GenericParam>) {
        self.r
            .defs
            .insert(generic_param.ast_id, DefKind::GenericParam, self.def_stack.last());

        visitor::walk_generic_param(self, generic_param);
    }

    fn visit_struct_field_def(&mut self, struct_field_def: &AstNode<FieldDef>) {
        self.r
            .defs
            .insert(struct_field_def.ast_id, DefKind::StructField, self.def_stack.last());

        visitor::walk_struct_field_def(self, struct_field_def);
    }

    fn visit_enum_variant(&mut self, enum_variant: &AstNode<EnumVariant>) {
        self.r
            .defs
            .insert(enum_variant.ast_id, DefKind::EnumVariant, self.def_stack.last());

        visitor::walk_enum_variant(self, enum_variant);
    }

    fn visit_assoc_item(&mut self, assoc_item: &AstNode<AssociatedItem>) {
        let def_kind = match &assoc_item.node {
            AssociatedItem::Fn(fn_sig, block) => {
                visitor::walk_fn_sig(self, fn_sig);

                if let Some(block) = block {
                    visitor::walk_block(self, block);
                }

                DefKind::AssocFn
            }
            AssociatedItem::Type(ty_alias) => {
                visitor::walk_assoc_ty_alias(self, ty_alias);
                DefKind::AssocTypeAlias
            }
        };

        self.r.defs.insert(assoc_item.ast_id, def_kind, self.def_stack.last());
    }
}
