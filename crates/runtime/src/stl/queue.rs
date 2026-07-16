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
                Some(Object::Queue(q)) => q.len() as i64,
                _ => 0,
            };
            Ok(ctx.query(base, "size", None, Value::Int(n), span))
        }
        "empty" => {
            let e = match ctx.heap.get(id) {
                Some(Object::Queue(q)) => q.is_empty(),
                _ => true,
            };
            Ok(ctx.query(base, "empty", None, Value::Bool(e), span))
        }
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
        "push" => {
            let v = args
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "push needs a value"))?;
            if let Some(Object::Queue(q)) = ctx.heap.get_mut(id) {
                q.push_back(v.clone());
            }
            ctx.modify(base, "queue::push", None, None, None, Some(v), span);
            Ok(Value::Void)
        }
        "pop" => {
            let old = if let Some(Object::Queue(q)) = ctx.heap.get_mut(id) {
                q.pop_front()
            } else {
                None
            };
            ctx.modify(base, "queue::pop", None, None, old, None, span);
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown queue method `{method}`"),
        )),
    }
}
