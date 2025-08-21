use crate::error::CompilerError;
use crate::session::ErrorHandler;
use miette::NamedSource;
use std::cell::RefCell;

pub struct Session {
    source: NamedSource<String>,
    pub error_handler: RefCell<ErrorHandler>,
}

impl Session {
    pub fn new(source: NamedSource<String>) -> Self {
        Self {
            source,
            error_handler: RefCell::new(ErrorHandler::new()),
        }
    }

    pub fn get_source(&self) -> String {
        self.source.inner().clone()
    }

    pub fn get_named_source(&self) -> NamedSource<String> {
        self.source.clone()
    }

    pub fn push_error(&self, error: CompilerError) {
        self.error_handler.borrow_mut().push_error(error)
    }

    pub fn emit_all(&self) {
        self.error_handler.borrow_mut().emit_all();
    }
}
