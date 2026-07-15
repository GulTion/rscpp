//! Per-container STL method implementations (one file per type).

mod map;
mod priority_queue;
mod queue;
mod set;
mod stack;
mod string;
mod vector;

use crate::error::RuntimeError;
use crate::event::Event;
use crate::value::{Heap, MapKey, ObjId, Value};
use rscpp_ast::Span;

type Result<T> = std::result::Result<T, RuntimeError>;

/// Shared access for STL methods: heap + event log.
pub struct Ctx<'a> {
    pub heap: &'a mut Heap,
    pub events: &'a mut Vec<Event>,
}

impl Ctx<'_> {
    pub fn emit(&mut self, e: Event) {
        self.events.push(e);
    }

    pub fn value_to_key(&self, v: &Value) -> Result<MapKey> {
        MapKey::from_value(v, |id| self.heap.string_value(id)).map_err(RuntimeError::new)
    }
}

/// Dispatch `obj.method(args)` for heap objects that are STL containers.
pub fn call_method(
    ctx: &mut Ctx<'_>,
    id: ObjId,
    base: Value,
    kind: &str,
    method: &str,
    args: &[Value],
    span: Span,
) -> Result<Value> {
    match kind {
        "vector" => vector::call(ctx, id, base, method, args, span),
        "string" => string::call(ctx, id, base, method, args, span),
        "map" | "unordered_map" => map::call(ctx, id, base, kind, method, args, span),
        "set" | "unordered_set" => set::call(ctx, id, base, method, args, span),
        "stack" => stack::call(ctx, id, base, method, args, span),
        "queue" => queue::call(ctx, id, base, method, args, span),
        "priority_queue" => priority_queue::call(ctx, id, base, method, args, span),
        "pair" => Err(RuntimeError::at(
            span,
            format!("pair has no method `{method}` (use .first / .second)"),
        )),
        other => Err(RuntimeError::at(
            span,
            format!("no STL methods for `{other}`"),
        )),
    }
}
