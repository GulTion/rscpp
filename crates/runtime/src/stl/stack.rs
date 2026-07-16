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
                Some(Object::Stack(s)) => s.len() as i64,
                _ => 0,
            };
            Ok(ctx.query(base, "size", None, Value::Int(n), span))
        }
        "empty" => {
            let e = match ctx.heap.get(id) {
                Some(Object::Stack(s)) => s.is_empty(),
                _ => true,
            };
            Ok(ctx.query(base, "empty", None, Value::Bool(e), span))
        }
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
        "push" => {
            let v = args
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "push needs a value"))?;
            if let Some(Object::Stack(s)) = ctx.heap.get_mut(id) {
                s.push(v.clone());
            }
            ctx.modify(base, "stack::push", None, None, None, Some(v), span);
            Ok(Value::Void)
        }
        "pop" => {
            let old = if let Some(Object::Stack(s)) = ctx.heap.get_mut(id) {
                s.pop()
            } else {
                None
            };
            ctx.modify(base, "stack::pop", None, None, old, None, span);
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown stack method `{method}`"),
        )),
    }
}
