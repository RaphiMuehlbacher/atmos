use crate::ast_lowerer::hir::Crate;
use crate::type_checker::ty::{Ty, TyVarId};
use crate::type_checker::type_collector::CollectedTypes;
use crate::Session;
use crate::ast_lowerer::hir::Crate;
use crate::type_checker::ty::{CollectedTypes, Ty, TyVarId};
use std::collections::HashMap;

pub struct InferCtxt {
    type_var_map: HashMap<TyVarId, Ty>,
}

pub struct TypeChecker<'hir> {
    session: &'hir Session,
    hir: &'hir Crate,
    collected_types: CollectedTypes,
}

impl<'hir> TypeChecker<'hir> {
    pub fn new(session: &'hir Session, hir: &'hir Crate, collected_types: CollectedTypes) -> Self {
        Self {
            session,
            hir,
            collected_types,
        }
    }

    pub fn check(&mut self) {}
}
