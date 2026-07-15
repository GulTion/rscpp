use crate::value::{ObjId, Value};
use rscpp_ast::Span;

/// Identity of a mutable storage location (for visualizer highlighting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot {
    Local {
        name: String,
    },
    Global {
        name: String,
    },
    Index {
        obj: ObjId,
        index: usize,
    },
    MapEntry {
        obj: ObjId,
        key: String,
    },
    Field {
        obj: ObjId,
        field: String,
    },
}

/// Runtime events for visualizers / debuggers. No UI coupling.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// About to execute a statement (primary stepping hook).
    Step {
        span: Span,
    },
    ScopeEnter {
        span: Span,
    },
    ScopeExit {
        span: Span,
    },
    VarCreate {
        name: String,
        value: Value,
        span: Span,
    },
    VarAssign {
        name: String,
        old: Option<Value>,
        value: Value,
        span: Span,
    },
    /// Structured lvalue write (locals, indices, fields).
    Write {
        slot: Slot,
        old: Option<Value>,
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
    /// Which branch of an `if` was taken (`then` = true).
    Branch {
        then_taken: bool,
        span: Span,
    },
    /// Start of a loop-body iteration.
    LoopIter {
        span: Span,
    },
    Compare {
        op: String,
        left: Value,
        right: Value,
        result: bool,
        span: Span,
    },
    /// Exchange of two slots (explicit `swap` or detected).
    Swap {
        a: Slot,
        b: Slot,
        value_a: Value,
        value_b: Value,
        span: Span,
    },
    /// Coarse container mutation (push/pop/clear). Prefer `Write` for index stores.
    ContainerMod {
        container: Value,
        kind: String,
        index: Option<usize>,
        old: Option<Value>,
        value: Option<Value>,
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
    /// Reference bound to a slot (when refs become real).
    RefBind {
        name: String,
        target: Slot,
        span: Span,
    },
    /// Pointer updated to a new address / object.
    PtrMove {
        name: String,
        to: Value,
        span: Span,
    },
}
