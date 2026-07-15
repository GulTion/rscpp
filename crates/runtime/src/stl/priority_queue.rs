use super::{Ctx, Result};
use crate::error::RuntimeError;
use crate::event::Event;
use crate::value::{Object, ObjId, Value};
use rscpp_ast::Span;

pub fn call(
    ctx: &mut Ctx<'_>,
    id: ObjId,
    base: Value,
    method: &str,
    args: &[Value],
    span: Span,
) -> Result<Value> {
    match method {
        "size" => match ctx.heap.get(id) {
            Some(Object::PriorityQueue(h)) => Ok(Value::Int(h.len() as i64)),
            _ => Ok(Value::Int(0)),
        },
        "empty" => match ctx.heap.get(id) {
            Some(Object::PriorityQueue(h)) => Ok(Value::Bool(h.is_empty())),
            _ => Ok(Value::Bool(true)),
        },
        "top" => match ctx.heap.get(id) {
            Some(Object::PriorityQueue(h)) => h
                .peek()
                .copied()
                .map(Value::Int)
                .ok_or_else(|| RuntimeError::at(span, "top on empty priority_queue")),
            _ => Err(RuntimeError::at(span, "not a priority_queue")),
        },
        "push" => {
            let n = args
                .first()
                .ok_or_else(|| RuntimeError::at(span, "push needs a value"))?
                .as_int()
                .map_err(RuntimeError::new)?;
            if let Some(Object::PriorityQueue(h)) = ctx.heap.get_mut(id) {
                h.push(n);
            }
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: "priority_queue::push".into(),
                index: None,
                old: None,
                value: Some(Value::Int(n)),
                span,
            });
            Ok(Value::Void)
        }
        "pop" => {
            let old = if let Some(Object::PriorityQueue(h)) = ctx.heap.get_mut(id) {
                h.pop().map(Value::Int)
            } else {
                None
            };
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: "priority_queue::pop".into(),
                index: None,
                old,
                value: None,
                span,
            });
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown priority_queue method `{method}`"),
        )),
    }
}
