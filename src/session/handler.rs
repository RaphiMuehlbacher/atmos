use crate::error::CompilerError;
use miette::Report;

#[derive(Default)]
pub struct ErrorHandler {
    errors: Vec<CompilerError>,
}

impl ErrorHandler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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

    #[must_use]
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}
