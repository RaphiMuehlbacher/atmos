#![allow(unused)]

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic)]
pub enum TypeCheckerError {
    #[error("cyclic type definition")]
    #[diagnostic(
        code(typechecker::cyclic_type_definition),
        help("type aliases must eventually resolve to a concrete type; `{name}` refers back to itself")
    )]
    CyclicTypeDefinition {
        #[source_code]
        src: NamedSource<String>,

        #[label("this type alias is cyclic")]
        span: SourceSpan,

        name: String,
    },
    #[error("expected {expected} argument, found {found} argument")]
    #[diagnostic(
        code(typechecker::generic_arg_kind_mismatch),
        help("generic parameter `{param}` expects a {expected} argument")
    )]
    GenericArgKindMismatch {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected a {expected} argument")]
        expected_span: SourceSpan,

        #[label("found a {found} argument")]
        found_span: SourceSpan,

        param: String,
        expected: String,
        found: String,
    },

    #[error("expected {expected} generic argument(s), found {found}")]
    #[diagnostic(
        code(typechecker::generic_arg_arity_mismatch),
        help("`{name}` declares {expected} generic parameter(s)")
    )]
    GenericArgArityMismatch {
        #[source_code]
        src: NamedSource<String>,

        #[label("found {found} argument(s)")]
        span: SourceSpan,

        name: String,
        expected: usize,
        found: usize,
    },

    #[error("generic arguments are not allowed on path segments other than the last")]
    #[diagnostic(
        code(typechecker::generic_args_on_leading_segment),
        help("move the generic arguments to the final path segment")
    )]
    GenericArgsOnLeadingSegment {
        #[source_code]
        src: NamedSource<String>,

        #[label("generic arguments not allowed here")]
        span: SourceSpan,
    },
    #[error("this function takes {expected} argument(s) but {found} argument(s) were supplied")]
    #[diagnostic(
        code(typechecker::fn_arg_arity_mismatch),
        help("provide the correct number of arguments")
    )]
    FnArgArityMismatch {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected {expected} argument(s)")]
        expected_span: SourceSpan,

        #[label("found {found} argument(s)")]
        found_span: SourceSpan,

        expected: usize,
        found: usize,
    },

    #[error("this struct has {expected} field(s) but {found} argument(s) were supplied")]
    #[diagnostic(
        code(typechecker::struct_arg_arity_mismatch),
        help("provide the correct number of arguments")
    )]
    StructArgArityMismatch {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected {expected} argument(s)")]
        expected_span: SourceSpan,

        #[label("found {found} argument(s)")]
        found_span: SourceSpan,

        expected: usize,
        found: usize,
    },

    #[error("expected function, found `{found}`")]
    #[diagnostic(code(typechecker::not_callable), help("only functions can be called"))]
    NotCallable {
        #[source_code]
        src: NamedSource<String>,

        #[label("call expression requires function")]
        span: SourceSpan,

        found: String,
    },
    #[error("no field named `{field}` on type `{ty}`")]
    #[diagnostic(
        code(typechecker::field_not_found),
        help("there is no field named `{field}` on this type")
    )]
    FieldNotFound {
        #[source_code]
        src: NamedSource<String>,

        #[label("field `{field}` not found")]
        span: SourceSpan,

        field: String,
        ty: String,
    },

    #[error("tuple index {index} out of bounds for tuple of length {len}")]
    #[diagnostic(
        code(typechecker::tuple_index_out_of_bounds),
        help("the tuple has {len} element(s) but index {index} was requested")
    )]
    TupleIndexOutOfBounds {
        #[source_code]
        src: NamedSource<String>,

        #[label("index {index} out of bounds")]
        span: SourceSpan,

        index: usize,
        len: usize,
    },
    #[error("duplicate field `{field}` in struct expression")]
    #[diagnostic(
        code(typechecker::duplicate_field),
        help("consider removing the duplicate field specification")
    )]
    DuplicateField {
        #[source_code]
        src: NamedSource<String>,

        #[label("field `{field}` first used here")]
        first_span: SourceSpan,

        #[label("and again here")]
        span: SourceSpan,

        field: String,
    },

    #[error("missing field `{field}` in struct expression")]
    #[diagnostic(code(typechecker::missing_field), help("all fields of the struct must be specified"))]
    MissingField {
        #[source_code]
        src: NamedSource<String>,

        #[label("missing field `{field}`")]
        span: SourceSpan,

        field: String,
    },
    #[error("mismatched types")]
    #[diagnostic(code(typechecker::type_mismatch))]
    TypeMismatch {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected `{expected}`")]
        expected_span: SourceSpan,

        #[label("found `{found}`")]
        found_span: SourceSpan,

        expected: String,
        found: String,
    },

    #[error("expected a value, found a type")]
    #[diagnostic(
        code(typechecker::expected_value_found_type),
        help("types can only be used in type position, not as values")
    )]
    ExpectedValueType {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected a value here")]
        span: SourceSpan,
    },

    #[error("cannot infer the type of this expression")]
    #[diagnostic(code(typechecker::cannot_infer_type), help("consider adding a type annotation"))]
    CannotInferType {
        #[source_code]
        src: NamedSource<String>,

        #[label("type annotations needed")]
        span: SourceSpan,
    },
}
