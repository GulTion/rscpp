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
    let Some(Object::Array { elems, n }) = ctx.heap.get(id).cloned() else {
        return Err(RuntimeError::at(span, "not an array"));
    };
    match method {
        "begin" | "cbegin" => Ok(Value::Int(0)),
        "end" | "cend" => Ok(Value::Int(n as i64)),
        "size" => Ok(ctx.size(id, base, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
        "front" => {
            let v = elems
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "front on empty array"))?;
            Ok(ctx.query(base, "front", Some(Value::Int(0)), v, span))
        }
        "back" => {
            let idx = n.saturating_sub(1) as i64;
            let v = elems
                .get(n.saturating_sub(1))
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "back on empty array"))?;
            Ok(ctx.query(base, "back", Some(Value::Int(idx)), v, span))
        }
        "at" => {
            let idx = Ctx::require_arg(args, method, span)?
                .as_int()
                .map_err(RuntimeError::new)? as usize;
            if idx >= n {
                return Err(RuntimeError::at(
                    span,
                    format!("array::at index {idx} out of range"),
                ));
            }
            let v = elems[idx].clone();
            Ok(ctx.query(base, "at", Some(Value::Int(idx as i64)), v, span))
        }
        "fill" => {
            let v = Ctx::require_arg(args, method, span)?;
            if let Some(Object::Array { elems, n }) = ctx.heap.get_mut(id) {
                for e in elems.iter_mut().take(*n) {
                    *e = v.clone();
                }
            }
            ctx.modify(base, "fill", None, None, None, Some(v), span);
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown array method `{method}`"),
        )),
    }
}
