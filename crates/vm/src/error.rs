use rscpp_ast::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmError {
    pub span: Option<Span>,
    pub message: String,
}

impl VmError {
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

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(
                f,
                "{} at bytes {}..{}",
                self.message, span.start, span.end
            )
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for VmError {}

impl From<rscpp_parser::ParseError> for VmError {
    fn from(e: rscpp_parser::ParseError) -> Self {
        Self::at(e.span, e.message)
    }
}

impl From<rscpp_runtime::RuntimeError> for VmError {
    fn from(e: rscpp_runtime::RuntimeError) -> Self {
        Self {
            span: e.span,
            message: e.message,
        }
    }
}
