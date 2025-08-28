use miette::SourceSpan;

pub trait SourceSpanExt {
    fn to(&self, other: SourceSpan) -> SourceSpan;
}

impl SourceSpanExt for SourceSpan {
    fn to(&self, other: SourceSpan) -> SourceSpan {
        let start = self.offset();
        let end = other.offset() + other.len();
        SourceSpan::from((start, end))
    }
}
