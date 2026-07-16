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
        "size" => Ok(ctx.size(id, base, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
        "front" => {
            let v = match ctx.heap.get(id) {
                Some(Object::Queue(q)) => q
                    .front()
                    .cloned()
                    .ok_or_else(|| RuntimeError::at(span, "front on empty queue"))?,
                _ => return Err(RuntimeError::at(span, "not a queue")),
            };
            Ok(ctx.query(base, "front", None, v, span))
        }
        "back" => {
            let v = match ctx.heap.get(id) {
                Some(Object::Queue(q)) => q
                    .back()
                    .cloned()
                    .ok_or_else(|| RuntimeError::at(span, "back on empty queue"))?,
                _ => return Err(RuntimeError::at(span, "not a queue")),
            };
            Ok(ctx.query(base, "back", None, v, span))
        }
        "push" | "emplace" => {
            let v = Ctx::require_arg(args, method, span)?;
            if let Some(Object::Queue(q)) = ctx.heap.get_mut(id) {
                q.push_back(v.clone());
            }
            Ok(ctx.pushed(base, format!("queue::{method}"), None, None, v, span))
        }
        "pop" => {
            let old = if let Some(Object::Queue(q)) = ctx.heap.get_mut(id) {
                q.pop_front()
            } else {
                None
            };
            Ok(ctx.popped(base, "queue::pop", None, old, span))
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown queue method `{method}`"),
        )),
    }
}
