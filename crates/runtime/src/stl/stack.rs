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
        "top" => {
            let v = match ctx.heap.get(id) {
                Some(Object::Stack(s)) => s
                    .last()
                    .cloned()
                    .ok_or_else(|| RuntimeError::at(span, "top on empty stack"))?,
                _ => return Err(RuntimeError::at(span, "not a stack")),
            };
            Ok(ctx.query(base, "top", None, v, span))
        }
        "push" | "emplace" => {
            let v = Ctx::require_arg(args, method, span)?;
            if let Some(Object::Stack(s)) = ctx.heap.get_mut(id) {
                s.push(v.clone());
            }
            Ok(ctx.pushed(base, format!("stack::{method}"), None, None, v, span))
        }
        "pop" => {
            let old = if let Some(Object::Stack(s)) = ctx.heap.get_mut(id) {
                s.pop()
            } else {
                None
            };
            Ok(ctx.popped(base, "stack::pop", None, old, span))
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown stack method `{method}`"),
        )),
    }
}
