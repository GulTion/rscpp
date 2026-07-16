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
        "clear" => Ok(ctx.clear(id, base, "set::clear", span)),
        "count" => {
            let key_v = Ctx::require_arg(args, "count", span)?;
            let key = ctx.value_to_key(&key_v)?;
            let c = match ctx.heap.get(id) {
                Some(Object::Set(s)) => s.contains(&key),
                Some(Object::UnorderedSet(s)) => s.contains(&key),
                _ => false,
            };
            Ok(ctx.query(
                base,
                "count",
                Some(key_v),
                Value::Int(if c { 1 } else { 0 }),
                span,
            ))
        }
        "insert" | "emplace" => {
            let key = ctx.value_to_key(&Ctx::require_arg(args, method, span)?)?;
            match ctx.heap.get_mut(id) {
                Some(Object::Set(s)) => {
                    s.insert(key.clone());
                }
                Some(Object::UnorderedSet(s)) => {
                    s.insert(key.clone());
                }
                _ => {}
            }
            let key_v = key.to_value();
            Ok(ctx.pushed(
                base,
                format!("set::{method}"),
                None,
                Some(key_v.clone()),
                key_v,
                span,
            ))
        }
        "erase" => {
            let key = ctx.value_to_key(&Ctx::require_arg(args, "erase", span)?)?;
            match ctx.heap.get_mut(id) {
                Some(Object::Set(s)) => {
                    s.remove(&key);
                }
                Some(Object::UnorderedSet(s)) => {
                    s.remove(&key);
                }
                _ => {}
            }
            Ok(ctx.popped(base, "set::erase", Some(key.to_value()), None, span))
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown set method `{method}`"),
        )),
    }
}
