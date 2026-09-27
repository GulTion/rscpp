use crate::diag::format_diagnostic;
use crate::span::Span;
use std::fmt;

/// A lexing failure with a source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub span: Span,
    pub message: String,
}

impl LexError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    pub fn format_with_source(&self, source: &str) -> String {
        format_diagnostic(source, self.span, "lex", &self.message)
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for LexError {}
