#![allow(unused)]

use crate::lexer::TokenKind;
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic)]
pub enum ParserError {
    #[error("Expected identifier, found `{found}`")]
    #[diagnostic(code(parser::expected_identifier), help("Expected an identifier"))]
    ExpectedIdentifier {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected identifier here")]
        span: SourceSpan,

        found: TokenKind,
    },

    #[error("Expected item, found `{found}`")]
    #[diagnostic(code(parser::expected_item), help("Expected an item"))]
    ExpectedItem {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected item here")]
        span: SourceSpan,

        found: TokenKind,
    },

    #[error("Expected associated item, found `{found}`")]
    #[diagnostic(code(parser::expected_associated_item), help("Expected associated item"))]
    ExpectedAssociatedItem {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected associated item here")]
        span: SourceSpan,

        found: TokenKind,
    },

    #[error("Expected expression, found `{found}`")]
    #[diagnostic(code(parser::expected_expression), help("Expected an expression"))]
    ExpectedExpression {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected expression here")]
        span: SourceSpan,

        found: TokenKind,
    },

    #[error("Expected pattern, found `{found}`")]
    #[diagnostic(code(parser::expected_pattern), help("Expected a pattern"))]
    ExpectedPattern {
        #[source_code]
        src: NamedSource<String>,

        #[label("expected pattern here")]
        span: SourceSpan,

        found: TokenKind,
    },

    #[error("Identifiers cannot start with a number")]
    #[diagnostic(
        code(parser::invalid_identifier_start),
        help("Identifiers must start with a letter or underscore (`_`)")
    )]
    InvalidIdentifierStart {
        #[source_code]
        src: NamedSource<String>,

        #[label("identifier starts with a number here")]
        span: SourceSpan,

        found: TokenKind,
    },

    #[error("Expected `{expected}`, found `{found}`")]
    #[diagnostic(code(parser::unexpected_token))]
    UnexpectedToken {
        #[source_code]
        src: NamedSource<String>,

        #[label("unexpected token found here")]
        span: SourceSpan,

        found: TokenKind,
        expected: TokenKind,
    },

    #[error("Unexpected closing delimiter: `{found_delimiter}`")]
    #[diagnostic(code(parser::unexpected_closing_delimiter))]
    UnexpectedClosingDelimiter {
        #[source_code]
        src: NamedSource<String>,

        #[label("unexpected closing delimiter here")]
        span: SourceSpan,

        found_delimiter: TokenKind,
    },
    #[error("Expected '{expected}' but found '{found}'")]
    #[diagnostic(code(parser::mismatched_delimiter))]
    MismatchedDelimiter {
        #[source_code]
        src: NamedSource<String>,

        #[label("mismatched closing delimiter")]
        closing_span: SourceSpan,

        #[label("opening delimiter here")]
        opening_span: SourceSpan,

        found: TokenKind,
        expected: TokenKind,
    },
    #[error("Unclosed delimiter")]
    #[diagnostic(code(parse::unclosed_delimiter), help("missing closing {delimiter}"))]
    UnclosedDelimiter {
        #[source_code]
        src: NamedSource<String>,

        #[label("unclosed delimiter here")]
        span: SourceSpan,

        delimiter: TokenKind,
    },

    #[error("Literal overflow: {message}")]
    #[diagnostic(
        code(parser::literal_overflow),
        help("The literal value is too large for the specified type")
    )]
    LiteralOverflow {
        #[source_code]
        src: NamedSource<String>,

        #[label("literal is too large here")]
        span: SourceSpan,

        message: String,
    },

    #[error("Invalid literal suffix `{suffix}` for {literal_type} literal")]
    #[diagnostic(
        code(parser::invalid_literal_suffix),
        help("Valid suffixes for {literal_type} literals are: {valid_suffixes}")
    )]
    InvalidLiteralSuffix {
        #[source_code]
        src: NamedSource<String>,

        #[label("invalid suffix here")]
        span: SourceSpan,

        suffix: String,
        literal_type: String,
        valid_suffixes: String,
    },
}
