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
    let Some(Object::Vector(elems)) = ctx.heap.get(id).cloned() else {
        return Err(RuntimeError::at(span, "not a vector"));
    };
    match method {
        "begin" | "end" | "cbegin" | "cend" => Ok(Value::Int(0)),
        "size" => Ok(ctx.size(id, base, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
        "push_back" | "emplace_back" => {
            let v = Ctx::require_arg(args, method, span)?;
            let idx = if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                e.push(v.clone());
                e.len() - 1
            } else {
                0
            };
            Ok(ctx.pushed(base, method, Some(idx), None, v, span))
        }
        "pop_back" => {
            let old = if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                e.pop()
            } else {
                None
            };
            Ok(ctx.popped(base, "pop_back", None, old, span))
        }
        "front" => {
            let v = elems
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "front on empty vector"))?;
            Ok(ctx.query(base, "front", Some(Value::Int(0)), v, span))
        }
        "back" => {
            let idx = elems.len().saturating_sub(1) as i64;
            let v = elems
                .last()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "back on empty vector"))?;
            Ok(ctx.query(base, "back", Some(Value::Int(idx)), v, span))
        }
        "clear" => Ok(ctx.clear(id, base, "clear", span)),
        "begin" | "end" => Ok(Value::Int(0)),
        _ => Err(RuntimeError::at(
            span,
            format!("unknown vector method `{method}`"),
        )),
    }
}
