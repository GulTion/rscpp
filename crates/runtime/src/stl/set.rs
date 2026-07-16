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
                Some(Object::Set(s)) => s.len(),
                Some(Object::UnorderedSet(s)) => s.len(),
                _ => 0,
            };
            Ok(ctx.query(base, "size", None, Value::Int(n as i64), span))
        }
        "empty" => {
            let e = match ctx.heap.get(id) {
                Some(Object::Set(s)) => s.is_empty(),
                Some(Object::UnorderedSet(s)) => s.is_empty(),
                _ => true,
            };
            Ok(ctx.query(base, "empty", None, Value::Bool(e), span))
        }
        "clear" => {
            match ctx.heap.get_mut(id) {
                Some(Object::Set(s)) => s.clear(),
                Some(Object::UnorderedSet(s)) => s.clear(),
                _ => {}
            }
            ctx.modify(base, "set::clear", None, None, None, None, span);
            Ok(Value::Void)
        }
        "count" => {
            let key_v = args
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "count needs a key"))?;
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
        "insert" => {
            let key = ctx.value_to_key(
                args.first()
                    .ok_or_else(|| RuntimeError::at(span, "insert needs a value"))?,
            )?;
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
            ctx.modify(
                base,
                "set::insert",
                None,
                Some(key_v.clone()),
                None,
                Some(key_v),
                span,
            );
            Ok(Value::Void)
        }
        "erase" => {
            let key = ctx.value_to_key(
                args.first()
                    .ok_or_else(|| RuntimeError::at(span, "erase needs a value"))?,
            )?;
            match ctx.heap.get_mut(id) {
                Some(Object::Set(s)) => {
                    s.remove(&key);
                }
                Some(Object::UnorderedSet(s)) => {
                    s.remove(&key);
                }
                _ => {}
            }
            ctx.modify(
                base,
                "set::erase",
                None,
                Some(key.to_value()),
                None,
                None,
                span,
            );
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown set method `{method}`"),
        )),
    }
}
