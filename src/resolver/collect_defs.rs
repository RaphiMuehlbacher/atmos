use crate::Resolver;
use crate::parser::ast::{AssociatedItem, AstNode, EnumVariant, FieldDef, GenericParam, Item};
use crate::resolver::defs::DefKind;
use crate::resolver::{DefId, visitor};

pub struct DefCollector<'a, 'r> {
    resolver: &'a mut Resolver<'r>,
    current_enum_def: Option<DefId>,
}

impl<'a, 'r> DefCollector<'a, 'r> {
    pub fn new(resolver: &'a mut Resolver<'r>) -> Self {
        Self {
            resolver,
            current_enum_def: None,
        }
    }
}

impl visitor::Visitor for DefCollector<'_, '_> {
    fn visit_item(&mut self, item: &AstNode<Item>) {
        let def_kind = DefKind::from(&item.node);

        let prev_item = self.current_enum_def.take();
        let def_id = self.resolver.defs.insert(item.ast_id, def_kind);
        self.current_enum_def = Some(def_id);

        visitor::walk_item(self, item);
        self.current_enum_def = prev_item
    }

    fn visit_generic_param(&mut self, generic_param: &AstNode<GenericParam>) {
        self.resolver.defs.insert(generic_param.ast_id, DefKind::GenericParam);

        visitor::walk_generic_param(self, generic_param);
    }

    fn visit_struct_field_def(&mut self, struct_field_def: &AstNode<FieldDef>) {
        self.resolver.defs.insert(struct_field_def.ast_id, DefKind::StructField);

        visitor::walk_struct_field_def(self, struct_field_def);
    }

    fn visit_enum_variant(&mut self, enum_variant: &AstNode<EnumVariant>) {
        self.resolver.defs.insert(
            enum_variant.ast_id,
            DefKind::EnumVariant {
                enum_def_id: self.current_enum_def.unwrap(),
            },
        );

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

        self.resolver.defs.insert(assoc_item.ast_id, def_kind);
    }
}
