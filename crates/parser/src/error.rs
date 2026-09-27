use rscpp_ast::Span;
use rscpp_lexer::format_diagnostic;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
}

impl ParseError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    pub fn format_with_source(&self, source: &str) -> String {
        format_diagnostic(source, self.span, "parse", &self.message)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

impl From<rscpp_lexer::LexError> for ParseError {
    fn from(e: rscpp_lexer::LexError) -> Self {
        Self::new(e.span, e.message)
    }
}
