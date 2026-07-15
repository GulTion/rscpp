use super::{Ctx, Result};
use crate::error::RuntimeError;
use crate::event::Event;
use crate::value::{MapKey, Object, ObjId, Value};
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
            Ok(Value::Int(n as i64))
        }
        "empty" => {
            let e = match ctx.heap.get(id) {
                Some(Object::Set(s)) => s.is_empty(),
                Some(Object::UnorderedSet(s)) => s.is_empty(),
                _ => true,
            };
            Ok(Value::Bool(e))
        }
        "clear" => {
            match ctx.heap.get_mut(id) {
                Some(Object::Set(s)) => s.clear(),
                Some(Object::UnorderedSet(s)) => s.clear(),
                _ => {}
            }
            Ok(Value::Void)
        }
        "count" => {
            let key = ctx.value_to_key(
                args.first()
                    .ok_or_else(|| RuntimeError::at(span, "count needs a key"))?,
            )?;
            let c = match ctx.heap.get(id) {
                Some(Object::Set(s)) => s.contains(&key),
                Some(Object::UnorderedSet(s)) => s.contains(&key),
                _ => false,
            };
            Ok(Value::Int(if c { 1 } else { 0 }))
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
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: "set::insert".into(),
                index: None,
                old: None,
                value: Some(Value::Int(match &key {
                    MapKey::Int(i) => *i,
                    _ => 1,
                })),
                span,
            });
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
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown set method `{method}`"),
        )),
    }
}
