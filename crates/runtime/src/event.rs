use crate::value::Value;
use rscpp_ast::Span;

/// Runtime events for visualizers / debuggers. No UI coupling.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    VarCreate {
        name: String,
        value: Value,
        span: Span,
    },
    VarAssign {
        name: String,
        value: Value,
        span: Span,
    },
    FnEnter {
        name: String,
        args: Vec<Value>,
        span: Span,
    },
    FnExit {
        name: String,
        ret: Value,
        span: Span,
    },
    Compare {
        op: String,
        left: Value,
        right: Value,
        result: bool,
        span: Span,
    },
    ContainerMod {
        container: Value,
        kind: String,
        detail: String,
        span: Span,
    },
    Alloc {
        id: u64,
        kind: String,
        span: Span,
    },
    Dealloc {
        id: u64,
        span: Span,
    },
}
