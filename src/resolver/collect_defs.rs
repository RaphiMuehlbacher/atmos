use crate::parser::ast::{AssociatedItem, AstNode, EnumVariant, GenericParam, Item, StructFieldDef};
use crate::resolver::defs::DefKind;
use crate::resolver::visitor;
use crate::Resolver;

pub struct DefCollector<'a, 'r> {
    pub resolver: &'a mut Resolver<'r>,
}

impl<'a, 'r> DefCollector<'a, 'r> {
    pub fn new(resolver: &'a mut Resolver<'r>) -> Self {
        Self { resolver }
    }
}

impl<'a, 'r> visitor::Visitor for DefCollector<'a, 'r> {
    fn visit_item(&mut self, item: &AstNode<Item>) {
        let def_kind = DefKind::from(&item.node);

        match item.node.ident() {
            None => self.resolver.defs.insert(None, def_kind, item.span, item.ast_id),
            Some(ident) => self.resolver.defs.insert_with_ident(ident, def_kind),
        };

        visitor::walk_item(self, item);
    }

    fn visit_generic_param(&mut self, generic_param: &AstNode<GenericParam>) {
        self.resolver
            .defs
            .insert_with_ident(&generic_param.node.ident, DefKind::TypeParam);

        visitor::walk_generic_param(self, generic_param);
    }

    fn visit_struct_field_def(&mut self, struct_field_def: &AstNode<StructFieldDef>) {
        self.resolver
            .defs
            .insert_with_ident(&struct_field_def.node.ident, DefKind::StructField);

        visitor::walk_struct_field_def(self, struct_field_def);
    }

    fn visit_enum_variant(&mut self, enum_variant: &AstNode<EnumVariant>) {
        self.resolver
            .defs
            .insert_with_ident(&enum_variant.node.ident, DefKind::EnumVariant);

        visitor::walk_enum_variant(self, enum_variant);
    }

    fn visit_assoc_item(&mut self, assoc_item: &AstNode<AssociatedItem>) {
        let (ident, def_kind) = match &assoc_item.node {
            AssociatedItem::Fn(sig, _) => (&sig.node.ident, DefKind::AssocFn),
            AssociatedItem::Type(ty) => (&ty.node.ident, DefKind::AssocTypeAlias),
        };

        self.resolver.defs.insert_with_ident(ident, def_kind);
    }
}
