use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic)]
pub enum LexerError {
    #[error("Unterminated multiline comment")]
    #[diagnostic(code(lexer::unterminated_comment))]
    UnterminatedComment {
        #[source_code]
        src: NamedSource<String>,
        #[label("Comment started here but was never closed")]
        span: SourceSpan,
    },

    #[error("Unexpected character: {character}")]
    #[diagnostic(
        help("This character isn't recognized by the lexer."),
        code(lexer::unexpected_char)
    )]
    UnexpectedCharacter {
        #[source_code]
        src: NamedSource<String>,

        #[label("unexpected `{character}` found here")]
        span: SourceSpan,

        character: char,
    },

    #[error("Invalid escape character: {character}")]
    #[diagnostic(
        help("if you meant to write a backslash escape it with another backslash"),
        code(lexer::unexpected_char)
    )]
    InvalidEscapeCharacter {
        #[source_code]
        src: NamedSource<String>,

        #[label("invalid escape character `{character}` found here")]
        span: SourceSpan,

        character: char,
    },
    #[error("Unterminated string literal")]
    #[diagnostic(
        help("Make sure all string literals are closed with a `\"`."),
        code(lexer::unterminated_string)
    )]
    UnterminatedString {
        #[source_code]
        src: NamedSource<String>,

        #[label("string starts here but never ends")]
        span: SourceSpan,
    },
}
