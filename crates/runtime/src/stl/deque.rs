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
    let Some(Object::Deque(elems)) = ctx.heap.get(id).cloned() else {
        return Err(RuntimeError::at(span, "not a deque"));
    };
    match method {
        "begin" | "cbegin" => Ok(Value::Int(0)),
        "end" | "cend" => Ok(Value::Int(elems.len() as i64)),
        "size" => Ok(ctx.size(id, base, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
        "clear" => Ok(ctx.clear(id, base, "clear", span)),
        "front" => {
            let v = elems
                .front()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "front on empty deque"))?;
            Ok(ctx.query(base, "front", Some(Value::Int(0)), v, span))
        }
        "back" => {
            let idx = elems.len().saturating_sub(1) as i64;
            let v = elems
                .back()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "back on empty deque"))?;
            Ok(ctx.query(base, "back", Some(Value::Int(idx)), v, span))
        }
        "at" => {
            let idx = Ctx::require_arg(args, method, span)?
                .as_int()
                .map_err(RuntimeError::new)? as usize;
            let v = elems.get(idx).cloned().ok_or_else(|| {
                RuntimeError::at(span, format!("deque::at index {idx} out of range"))
            })?;
            Ok(ctx.query(base, "at", Some(Value::Int(idx as i64)), v, span))
        }
        "push_back" | "emplace_back" => {
            let v = Ctx::require_arg(args, method, span)?;
            if let Some(Object::Deque(e)) = ctx.heap.get_mut(id) {
                e.push_back(v.clone());
            }
            Ok(ctx.pushed(base, method, None, None, v, span))
        }
        "push_front" | "emplace_front" => {
            let v = Ctx::require_arg(args, method, span)?;
            if let Some(Object::Deque(e)) = ctx.heap.get_mut(id) {
                e.push_front(v.clone());
            }
            Ok(ctx.pushed(base, method, Some(0), None, v, span))
        }
        "pop_back" => {
            let old = if let Some(Object::Deque(e)) = ctx.heap.get_mut(id) {
                e.pop_back()
            } else {
                None
            };
            Ok(ctx.popped(base, "pop_back", None, old, span))
        }
        "pop_front" => {
            let old = if let Some(Object::Deque(e)) = ctx.heap.get_mut(id) {
                e.pop_front()
            } else {
                None
            };
            Ok(ctx.popped(base, "pop_front", None, old, span))
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown deque method `{method}`"),
        )),
    }
}
