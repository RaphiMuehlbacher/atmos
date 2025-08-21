use crate::error::CompilerError;
use miette::Report;

pub struct ErrorHandler {
    errors: Vec<CompilerError>,
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }

    pub fn push_error(&mut self, error: CompilerError) {
        self.errors.push(error)
    }

    pub fn emit_all(&self) {
        for error in &self.errors {
            let report = Report::new(error.clone());
            println!("{report:?}");
        }
    }
}
