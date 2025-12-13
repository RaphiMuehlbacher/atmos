#![allow(unused)]

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
    #[error("Variable `{name}` is not bound in all alternatives")]
    #[diagnostic(
        code(resolver::variable_not_bound_in_pattern),
        help("Ensure `{name}` is bound in every alternative of the or-pattern")
    )]
    VariableNotBoundInPattern {
        #[source_code]
        src: NamedSource<String>,

        #[label("variable not bound in this alternative")]
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
    #[error("there are too many leading `super` keywords")]
    #[diagnostic(code(resolver::super_beyond_root))]
    SuperBeyondRoot {
        #[source_code]
        src: NamedSource<String>,

        #[label("beyond crate root")]
        span: SourceSpan,
    },
    #[error("`Self` is only available in impls, traits, and type definitions")]
    #[diagnostic(
        code(resolver::self_outside_impl),
        help("`Self` can only be used inside `impl` blocks, `trait` definitions, or type definitions")
    )]
    SelfOutsideImpl {
        #[source_code]
        src: NamedSource<String>,

        #[label("`Self` is not valid here")]
        span: SourceSpan,
    },
    #[error("`Self` cannot be used as a binding")]
    #[diagnostic(
        code(resolver::self_as_binding),
        help("`Self` is reserved for referring to the implementing type")
    )]
    SelfAsBinding {
        #[source_code]
        src: NamedSource<String>,

        #[label("cannot bind to `Self`")]
        span: SourceSpan,
    },
}
