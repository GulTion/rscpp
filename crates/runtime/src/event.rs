//! Runtime events for visualizers / debuggers. No UI coupling.
//!
//! **JSON contract (visualizer source of truth):** `docs/events.md` in the repo root docs.
//! Keep this module’s `Serialize` shapes aligned with that file.

use crate::value::{MapKey, ObjId, Value};
use rscpp_ast::Span;
use serde::Serialize;

/// Identity of a mutable storage location (for visualizer highlighting).
///
/// See `docs/events.md` § Slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Slot {
    Local { name: String },
    Global { name: String },
    Object { obj: ObjId },
    Index { obj: ObjId, index: usize },
    MapEntry { obj: ObjId, key: String },
    Field { obj: ObjId, field: String },
}

/// One map/set entry at `Alloc` time (keys as `MapKey`, values optional for sets).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AllocEntry {
    pub key: MapKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

/// Runtime events for visualizers / debuggers. No UI coupling.
///
/// Serialized as `{ "kind": "<Variant>", ... }`. Full field guide: `docs/events.md`.
///
/// **`call_id` / `parent_id` appear only on `FnEnter` / `FnExit`.** The root activation
/// (`main` or top-level `run_method`) uses `call_id: 0` and `parent_id: null`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind")]
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
    VarDestroy {
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
        /// Unique id for this activation (monotonic). Root / `main` is `0`.
        call_id: u64,
        /// Caller's `call_id`; `None` for top-level entry (e.g. `main`).
        parent_id: Option<u64>,
        args: Vec<Value>,
        span: Span,
    },
    FnExit {
        name: String,
        call_id: u64,
        parent_id: Option<u64>,
        ret: Value,
        span: Span,
    },
    /// Which branch of an `if` was taken (`then` = true).
    Branch {
        then_taken: bool,
        span: Span,
    },
    /// Start of a loop-body iteration (`loop_id` = runtime instance).
    LoopIter {
        loop_id: u64,
        span: Span,
    },
    /// `continue;` targeting the innermost open loop instance.
    Continue {
        loop_id: u64,
        span: Span,
    },
    /// `break;` targeting the innermost open loop instance.
    Break {
        loop_id: u64,
        span: Span,
    },
    /// Loop instance finished (`reason`: `exhausted` | `break` | `return`).
    LoopEnd {
        loop_id: u64,
        reason: String,
        span: Span,
    },
    Compare {
        op: String,
        left: Value,
        right: Value,
        result: bool,
        span: Span,
    },
    /// Builtin chooser (`min` / `max`) selected one input argument.
    BuiltinSelect {
        name: String,
        args: Vec<Value>,
        chosen: usize,
        value: Value,
        /// Span of the selected argument.
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
    /// Bulk ops (`sort`, `reverse`, `iota`, `partial_sum`) include post-state `elems`
    /// (same shape as `Alloc.elems`). Incremental ops leave `elems` empty.
    ContainerMod {
        container: Value,
        #[serde(rename = "op")]
        kind: String,
        index: Option<usize>,
        /// Map/set key when relevant.
        key: Option<Value>,
        old: Option<Value>,
        value: Option<Value>,
        /// After-state sequence snapshot for bulk mutations; empty otherwise.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        elems: Vec<Value>,
        span: Span,
    },
    /// Read-only container query (`count`, `size`, `empty`, `top`, `index`, …).
    ContainerLookup {
        container: Value,
        #[serde(rename = "op")]
        kind: String,
        /// Lookup key / index when relevant.
        key: Option<Value>,
        result: Value,
        span: Span,
    },
    Alloc {
        id: u64,
        #[serde(rename = "type_name")]
        kind: String,
        /// Logical length at allocation time (see `docs/events.md`).
        size: usize,
        /// Element snapshot for sequences (vector/stack/queue/string/pair/…).
        elems: Vec<Value>,
        /// Key/value snapshot for map/set (empty for other types).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        entries: Vec<AllocEntry>,
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
