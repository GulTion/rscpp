use super::{Ctx, Result};
use crate::error::RuntimeError;
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
            Ok(ctx.query(base, "size", None, Value::Int(n as i64), span))
        }
        "empty" => {
            let e = match ctx.heap.get(id) {
                Some(Object::Map(m)) => m.is_empty(),
                Some(Object::UnorderedMap(m)) => m.is_empty(),
                _ => true,
            };
            Ok(ctx.query(base, "empty", None, Value::Bool(e), span))
        }
        "clear" => {
            match ctx.heap.get_mut(id) {
                Some(Object::Map(m)) => m.clear(),
                Some(Object::UnorderedMap(m)) => m.clear(),
                _ => {}
            }
            ctx.modify(base, format!("{kind}::clear"), None, None, None, None, span);
            Ok(Value::Void)
        }
        "count" => {
            let key_v = args
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "count needs a key"))?;
            let key = ctx.value_to_key(&key_v)?;
            let c = match ctx.heap.get(id) {
                Some(Object::Map(m)) => m.contains_key(&key),
                Some(Object::UnorderedMap(m)) => m.contains_key(&key),
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
            ctx.modify(
                base,
                format!("{kind}::erase"),
                None,
                Some(key.to_value()),
                old,
                None,
                span,
            );
            Ok(Value::Void)
        }
        "insert" => {
            let (k, v) = pair_or_args_as_kv(ctx, args, span)?;
            match ctx.heap.get_mut(id) {
                Some(Object::Map(m)) => {
                    m.insert(k.clone(), v.clone());
                }
                Some(Object::UnorderedMap(m)) => {
                    m.insert(k.clone(), v.clone());
                }
                _ => {}
            }
            ctx.modify(
                base,
                format!("{kind}::insert"),
                None,
                Some(k.to_value()),
                None,
                Some(v),
                span,
            );
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
