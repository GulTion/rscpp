use super::{Ctx, Result};
use crate::error::RuntimeError;
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
        "size" => {
            let n = match ctx.heap.get(id) {
                Some(Object::PriorityQueue(h)) => h.len() as i64,
                _ => 0,
            };
            Ok(ctx.query(base, "size", None, Value::Int(n), span))
        }
        "empty" => {
            let e = match ctx.heap.get(id) {
                Some(Object::PriorityQueue(h)) => h.is_empty(),
                _ => true,
            };
            Ok(ctx.query(base, "empty", None, Value::Bool(e), span))
        }
        "top" => {
            let v = match ctx.heap.get(id) {
                Some(Object::PriorityQueue(h)) => h
                    .peek()
                    .copied()
                    .map(Value::Int)
                    .ok_or_else(|| RuntimeError::at(span, "top on empty priority_queue"))?,
                _ => return Err(RuntimeError::at(span, "not a priority_queue")),
            };
            Ok(ctx.query(base, "top", None, v, span))
        }
        "push" => {
            let n = args
                .first()
                .ok_or_else(|| RuntimeError::at(span, "push needs a value"))?
                .as_int()
                .map_err(RuntimeError::new)?;
            if let Some(Object::PriorityQueue(h)) = ctx.heap.get_mut(id) {
                h.push(n);
            }
            ctx.modify(base, "priority_queue::push", None, None, None, Some(Value::Int(n)), span);
            Ok(Value::Void)
        }
        "pop" => {
            let old = if let Some(Object::PriorityQueue(h)) = ctx.heap.get_mut(id) {
                h.pop().map(Value::Int)
            } else {
                None
            };
            ctx.modify(base, "priority_queue::pop", None, None, old, None, span);
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown priority_queue method `{method}`"),
        )),
    }
}
