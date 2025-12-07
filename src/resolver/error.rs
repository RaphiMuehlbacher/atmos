use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic)]
pub enum ResolverError {
    #[error("Cannot find `{name}` in this scope")]
    #[diagnostic(
        code(resolver::name_not_found),
        help("Make sure `{name}` is defined before using it")
    )]
    NameNotFound {
        #[source_code]
        src: NamedSource<String>,

        #[label("not found in this scope")]
        span: SourceSpan,

        name: String,
    },

    #[error("The name `{name}` is defined multiple times")]
    #[diagnostic(
        code(resolver::duplicate_definition),
        help("Consider renaming one of the definitions")
    )]
    DuplicateDefinition {
        #[source_code]
        src: NamedSource<String>,

        #[label("redefined here")]
        span: SourceSpan,

        name: String,
    },

    #[error("Cannot resolve path `{path}`")]
    #[diagnostic(code(resolver::unresolved_path))]
    UnresolvedPath {
        #[source_code]
        src: NamedSource<String>,

        #[label("unresolved path")]
        span: SourceSpan,

        path: String,
    },
}
