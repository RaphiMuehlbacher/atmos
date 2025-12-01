use crate::parser::ast::{AssociatedItem, AstNode, EnumVariant, GenericParam, Item, StructFieldDef};
use crate::resolver::defs::DefKind;
use crate::resolver::visitor;
use crate::Resolver;

pub struct DefCollector<'a, 'r> {
    resolver: &'a mut Resolver<'r>,
}

impl<'a, 'r> DefCollector<'a, 'r> {
    pub fn new(resolver: &'a mut Resolver<'r>) -> Self {
        Self { resolver }
    }
}

impl<'a, 'r> visitor::Visitor for DefCollector<'a, 'r> {
    fn visit_item(&mut self, item: &AstNode<Item>) {
        let def_kind = DefKind::from(&item.node);
        self.resolver.defs.insert(item.ast_id, def_kind);
        visitor::walk_item(self, item);
    }

    fn visit_generic_param(&mut self, generic_param: &AstNode<GenericParam>) {
        self.resolver.defs.insert(generic_param.ast_id, DefKind::TypeParam);

        visitor::walk_generic_param(self, generic_param);
    }

    fn visit_struct_field_def(&mut self, struct_field_def: &AstNode<StructFieldDef>) {
        self.resolver.defs.insert(struct_field_def.ast_id, DefKind::StructField);

        visitor::walk_struct_field_def(self, struct_field_def);
    }

    fn visit_enum_variant(&mut self, enum_variant: &AstNode<EnumVariant>) {
        self.resolver.defs.insert(enum_variant.ast_id, DefKind::EnumVariant);

        visitor::walk_enum_variant(self, enum_variant);
    }

    fn visit_assoc_item(&mut self, assoc_item: &AstNode<AssociatedItem>) {
        let def_kind = match &assoc_item.node {
            AssociatedItem::Fn(_, _) => DefKind::AssocFn,
            AssociatedItem::Type(_) => DefKind::AssocTypeAlias,
        };

        self.resolver.defs.insert(assoc_item.ast_id, def_kind);
    }
}
