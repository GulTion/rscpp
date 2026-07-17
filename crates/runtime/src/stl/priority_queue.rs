use super::{Ctx, Result};
use crate::error::RuntimeError;
use crate::value::{ObjId, Object, Value};
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
        "size" => Ok(ctx.size(id, base, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
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
        "push" | "emplace" => {
            let n = Ctx::require_arg(args, method, span)?
                .as_int()
                .map_err(RuntimeError::new)?;
            if let Some(Object::PriorityQueue(h)) = ctx.heap.get_mut(id) {
                h.push(n);
            }
            Ok(ctx.pushed(
                base,
                format!("priority_queue::{method}"),
                None,
                None,
                Value::Int(n),
                span,
            ))
        }
        "pop" => {
            let old = if let Some(Object::PriorityQueue(h)) = ctx.heap.get_mut(id) {
                h.pop().map(Value::Int)
            } else {
                None
            };
            Ok(ctx.popped(base, "priority_queue::pop", None, old, span))
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown priority_queue method `{method}`"),
        )),
    }
}
