use super::{Ctx, Result};
use crate::error::RuntimeError;
use crate::event::Event;
use crate::value::{MapKey, Object, ObjId, Value};
use rscpp_ast::Span;

pub fn call(
    ctx: &mut Ctx<'_>,
    id: ObjId,
    base: Value,
    kind: &str,
    method: &str,
    args: &[Value],
    span: Span,
) -> Result<Value> {
    match method {
        "size" => {
            let n = match ctx.heap.get(id) {
                Some(Object::Map(m)) => m.len(),
                Some(Object::UnorderedMap(m)) => m.len(),
                _ => 0,
            };
            Ok(Value::Int(n as i64))
        }
        "empty" => {
            let e = match ctx.heap.get(id) {
                Some(Object::Map(m)) => m.is_empty(),
                Some(Object::UnorderedMap(m)) => m.is_empty(),
                _ => true,
            };
            Ok(Value::Bool(e))
        }
        "clear" => {
            match ctx.heap.get_mut(id) {
                Some(Object::Map(m)) => m.clear(),
                Some(Object::UnorderedMap(m)) => m.clear(),
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
                Some(Object::Map(m)) => m.contains_key(&key),
                Some(Object::UnorderedMap(m)) => m.contains_key(&key),
                _ => false,
            };
            Ok(Value::Int(if c { 1 } else { 0 }))
        }
        "erase" => {
            let key = ctx.value_to_key(
                args.first()
                    .ok_or_else(|| RuntimeError::at(span, "erase needs a key"))?,
            )?;
            let old = match ctx.heap.get_mut(id) {
                Some(Object::Map(m)) => m.remove(&key),
                Some(Object::UnorderedMap(m)) => m.remove(&key),
                _ => None,
            };
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: format!("{kind}::erase"),
                index: None,
                old,
                value: None,
                span,
            });
            Ok(Value::Void)
        }
        "insert" => {
            let (k, v) = pair_or_args_as_kv(ctx, args, span)?;
            match ctx.heap.get_mut(id) {
                Some(Object::Map(m)) => {
                    m.insert(k, v.clone());
                }
                Some(Object::UnorderedMap(m)) => {
                    m.insert(k, v.clone());
                }
                _ => {}
            }
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: format!("{kind}::insert"),
                index: None,
                old: None,
                value: Some(v),
                span,
            });
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown {kind} method `{method}`"),
        )),
    }
}

fn pair_or_args_as_kv(ctx: &Ctx<'_>, args: &[Value], span: Span) -> Result<(MapKey, Value)> {
    if args.len() == 1 {
        let Value::Object(pid) = &args[0] else {
            return Err(RuntimeError::at(span, "insert expects pair or (key,value)"));
        };
        match ctx.heap.get(*pid) {
            Some(Object::Pair { first, second }) => {
                Ok((ctx.value_to_key(first)?, second.clone()))
            }
            _ => Err(RuntimeError::at(span, "insert expects a pair")),
        }
    } else if args.len() >= 2 {
        Ok((ctx.value_to_key(&args[0])?, args[1].clone()))
    } else {
        Err(RuntimeError::at(span, "insert needs arguments"))
    }
}
