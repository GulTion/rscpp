use rscpp_ast::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeError {
    pub span: Option<Span>,
    pub message: String,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            span: None,
            message: message.into(),
        }
    }

    pub fn at(span: Span, message: impl Into<String>) -> Self {
        Self {
            span: Some(span),
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(f, "{} at bytes {}..{}", self.message, span.start, span.end)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<rscpp_parser::ParseError> for RuntimeError {
    fn from(e: rscpp_parser::ParseError) -> Self {
        Self::at(e.span, e.message)
    }
}
