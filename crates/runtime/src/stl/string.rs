use super::{Ctx, Result};
use crate::error::RuntimeError;
use crate::value::{Object, ObjId, Value};
use rscpp_ast::Span;

pub fn call(
    ctx: &mut Ctx<'_>,
    id: ObjId,
    _base: Value,
    method: &str,
    args: &[Value],
    span: Span,
) -> Result<Value> {
    let Some(Object::String(s)) = ctx.heap.get(id).cloned() else {
        return Err(RuntimeError::at(span, "not a string"));
    };
    match method {
        "size" | "length" => Ok(Value::Int(s.len() as i64)),
        "empty" => Ok(Value::Bool(s.is_empty())),
        "clear" => {
            if let Some(Object::String(s)) = ctx.heap.get_mut(id) {
                s.clear();
            }
            Ok(Value::Void)
        }
        "push_back" => {
            let ch = match args.first() {
                Some(Value::Char(c)) => *c,
                Some(v) => {
                    return Err(RuntimeError::at(
                        span,
                        format!("string::push_back expects char, got {v}"),
                    ))
                }
                None => return Err(RuntimeError::at(span, "push_back needs an argument")),
            };
            if let Some(Object::String(s)) = ctx.heap.get_mut(id) {
                s.push(ch);
            }
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown string method `{method}`"),
        )),
    }
}
