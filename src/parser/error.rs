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
    #[error("Expected '{expected:?}' but found '{found:?}'")]
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
    #[diagnostic(code(parse::unclosed_delimiter), help("missing closing {delimiter:?}"))]
    UnclosedDelimiter {
        #[source_code]
        src: NamedSource<String>,

        #[label("unclosed delimiter here")]
        span: SourceSpan,

        delimiter: TokenKind,
    },
}
