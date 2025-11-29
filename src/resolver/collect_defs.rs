use crate::parser::ast::Item;
use crate::resolver::visitor::Visitor;
use crate::Resolver;

pub struct DefCollector<'a, 'r> {
    pub resolver: &'a mut Resolver<'r>,
}

impl<'a, 'r> DefCollector<'a, 'r> {
    pub fn new(resolver: &'a mut Resolver<'r>) -> Self {
        Self { resolver }
    }
}

impl<'a, 'r> Visitor for DefCollector<'a, 'r> {
    fn visit_item(&mut self, item: &Item) {}
}
