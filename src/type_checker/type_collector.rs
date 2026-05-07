use crate::ast_lowerer::hir::Crate;
use crate::Session;

pub struct TypeChecker<'hir> {
    session: &'hir Session,
    hir_krate: &'hir Crate,
}

impl<'hir> TypeChecker<'hir> {
    pub fn new(session: &'hir Session, hir_krate: &'hir Crate) -> Self {
        Self { session, hir_krate }
    }
}
