//! Per-container STL method implementations (one file per type).

mod array;
mod deque;
mod list;
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

    /// Emit one `ContainerLookup` and return `result` (DRY for size/count/empty/top/index/…).
    pub fn query(
        &mut self,
        container: Value,
        op: impl Into<String>,
        key: Option<Value>,
        result: Value,
        span: Span,
    ) -> Value {
        self.emit(Event::ContainerLookup {
            container,
            kind: op.into(),
            key,
            result: result.clone(),
            span,
        });
        result
    }

    /// Emit one `ContainerMod` (DRY for push/pop/insert/erase/…).
    pub fn modify(
        &mut self,
        container: Value,
        op: impl Into<String>,
        index: Option<usize>,
        key: Option<Value>,
        old: Option<Value>,
        value: Option<Value>,
        span: Span,
    ) {
        self.emit(Event::ContainerMod {
            container,
            kind: op.into(),
            index,
            key,
            old,
            value,
            elems: vec![],
            span,
        });
    }

    pub fn size(&mut self, id: ObjId, base: Value, span: Span) -> Value {
        let n = self.heap.get(id).map(|o| o.len()).unwrap_or(0) as i64;
        self.query(base, "size", None, Value::Int(n), span)
    }

    pub fn length_alias(&mut self, id: ObjId, base: Value, op: &str, span: Span) -> Value {
        let n = self.heap.get(id).map(|o| o.len()).unwrap_or(0) as i64;
        self.query(base, op, None, Value::Int(n), span)
    }

    pub fn empty(&mut self, id: ObjId, base: Value, span: Span) -> Value {
        let e = self.heap.get(id).map(|o| o.is_empty()).unwrap_or(true);
        self.query(base, "empty", None, Value::Bool(e), span)
    }

    pub fn clear(&mut self, id: ObjId, base: Value, op: impl Into<String>, span: Span) -> Value {
        if let Some(o) = self.heap.get_mut(id) {
            o.clear();
        }
        self.modify(base, op, None, None, None, None, span);
        Value::Void
    }

    pub fn require_arg(args: &[Value], method: &str, span: Span) -> Result<Value> {
        args.first()
            .cloned()
            .ok_or_else(|| RuntimeError::at(span, format!("{method} needs an argument")))
    }

    /// After mutating the container: emit push-style `ContainerMod`, return void.
    pub fn pushed(
        &mut self,
        base: Value,
        op: impl Into<String>,
        index: Option<usize>,
        key: Option<Value>,
        value: Value,
        span: Span,
    ) -> Value {
        self.modify(base, op, index, key, None, Some(value), span);
        Value::Void
    }

    /// After mutating the container: emit pop/erase-style `ContainerMod`, return void.
    pub fn popped(
        &mut self,
        base: Value,
        op: impl Into<String>,
        key: Option<Value>,
        old: Option<Value>,
        span: Span,
    ) -> Value {
        self.modify(base, op, None, key, old, None, span);
        Value::Void
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
        "deque" => deque::call(ctx, id, base, method, args, span),
        "list" => list::call(ctx, id, base, method, args, span),
        "array" => array::call(ctx, id, base, method, args, span),
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
