use rscpp_ast::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemaError {
    pub span: Span,
    pub message: String,
}

impl SemaError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

impl fmt::Display for SemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for SemaError {}
