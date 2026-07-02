#![allow(unused)]

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic)]
pub enum TypeCheckerError {
    #[error("Cyclic type definition involving `{name}`")]
    #[diagnostic(
        code(typechecker::cyclic_type_definition),
        help(
            "Type aliases must eventually expand to a concrete type. `{name}` refers back to itself."
        )
    )]
    CyclicTypeDefinition {
        #[source_code]
        src: NamedSource<String>,

        #[label("this type alias is cyclic")]
        span: SourceSpan,

        name: String,
    },
}
