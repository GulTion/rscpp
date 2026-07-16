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
        "size" => Ok(ctx.query(base, "size", None, Value::Int(elems.len() as i64), span)),
        "empty" => Ok(ctx.query(base, "empty", None, Value::Bool(elems.is_empty()), span)),
        "push_back" => {
            let v = args
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "push_back needs an argument"))?;
            let idx = if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                e.push(v.clone());
                e.len() - 1
            } else {
                0
            };
            ctx.modify(base, "push_back", Some(idx), None, None, Some(v), span);
            Ok(Value::Void)
        }
        "pop_back" => {
            let old = if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                e.pop()
            } else {
                None
            };
            ctx.modify(base, "pop_back", None, None, old, None, span);
            Ok(Value::Void)
        }
        "clear" => {
            if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                e.clear();
            }
            ctx.modify(base, "clear", None, None, None, None, span);
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown vector method `{method}`"),
        )),
    }
}
