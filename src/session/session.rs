use crate::ast_lowerer::hir;
use crate::error::CompilerError;
use crate::lexer;
use crate::parser::ast;
use crate::resolver::{DefId, ribs};
use crate::session::ErrorHandler;
use clap::ValueEnum;
use miette::NamedSource;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

pub struct Session {
    source: NamedSource<String>,
    pub error_handler: RefCell<ErrorHandler>,
    pub flags: HashSet<OutputType>,
}

impl Session {
    #[must_use]
    pub fn new(source: NamedSource<String>, flags: HashSet<OutputType>) -> Self {
        Self {
            source,
            error_handler: RefCell::new(ErrorHandler::new()),
            flags,
        }
    }

    #[must_use]
    pub fn get_source(&self) -> String {
        self.source.inner().clone()
    }

    #[must_use]
    pub fn get_named_source(&self) -> NamedSource<String> {
        self.source.clone()
    }

    pub fn push_error(&self, error: CompilerError) {
        self.error_handler.borrow_mut().push_error(error);
    }

    pub fn emit_all(&self) {
        self.error_handler.borrow_mut().emit_all();
    }
}

#[derive(Clone, Default)]
pub struct Output {
    pub tokens: Option<Vec<lexer::Token>>,
    pub ast: Option<ast::Crate>,
    pub hir: Option<hir::Crate>,
    pub ast_to_def: HashMap<ast::AstId, DefId>,
    pub resolutions: HashMap<ast::AstId, ribs::Res>,
}

#[derive(Clone, Copy, ValueEnum, Hash, PartialEq, Eq)]
pub enum OutputType {
    Tokens,
    Ast,
    Hir,
}
