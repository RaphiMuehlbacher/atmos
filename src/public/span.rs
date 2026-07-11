use miette::SourceSpan;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Span {
    pub offset: usize,
    pub length: usize,
}

impl From<SourceSpan> for Span {
    fn from(s: SourceSpan) -> Self {
        Self {
            offset: s.offset(),
            length: s.len(),
        }
    }
}
