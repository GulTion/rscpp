use super::{Ctx, Result};
use crate::error::RuntimeError;
use crate::value::{MapKey, ObjId, Object, Value};
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
        "size" => Ok(ctx.size(id, base, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
        "clear" => Ok(ctx.clear(id, base, format!("{kind}::clear"), span)),
        // ponytail: iterator stubs — end==0; find returns 1 if present else 0 (supports find!=end only)
        "begin" | "end" | "cbegin" | "cend" | "rbegin" | "rend" | "crbegin" | "crend" => {
            Ok(Value::Int(0))
        }
        "lower_bound" | "upper_bound" => {
            if kind != "map" {
                return Err(RuntimeError::at(
                    span,
                    format!("{method} only on ordered map"),
                ));
            }
            let key_v = Ctx::require_arg(args, method, span)?;
            let key = ctx.value_to_key(&key_v)?;
            let keys: Vec<MapKey> = match ctx.heap.get(id) {
                Some(Object::Map(m)) => m.keys().cloned().collect(),
                _ => {
                    return Err(RuntimeError::at(span, "not a map"));
                }
            };
            let idx = if method == "lower_bound" {
                keys.iter().position(|k| *k >= key).unwrap_or(keys.len())
            } else {
                keys.iter().position(|k| *k > key).unwrap_or(keys.len())
            };
            Ok(ctx.query(base, method, Some(key_v), Value::Int(idx as i64), span))
        }
        "equal_range" => {
            if kind != "map" {
                return Err(RuntimeError::at(span, "equal_range only on ordered map"));
            }
            let key_v = Ctx::require_arg(args, method, span)?;
            let key = ctx.value_to_key(&key_v)?;
            let keys: Vec<MapKey> = match ctx.heap.get(id) {
                Some(Object::Map(m)) => m.keys().cloned().collect(),
                _ => return Err(RuntimeError::at(span, "not a map")),
            };
            let lo = keys.iter().position(|k| *k >= key).unwrap_or(keys.len());
            let hi = keys.iter().position(|k| *k > key).unwrap_or(keys.len());
            let pid = ctx.heap.alloc(Object::Pair {
                first: Value::Int(lo as i64),
                second: Value::Int(hi as i64),
            });
            Ok(ctx.query(base, method, Some(key_v), Value::Object(pid), span))
        }
        "find" => {
            let key_v = Ctx::require_arg(args, "find", span)?;
            let key = ctx.value_to_key(&key_v)?;
            let found = match ctx.heap.get(id) {
                Some(Object::Map(m)) => m.contains_key(&key),
                Some(Object::UnorderedMap(m)) => m.contains_key(&key),
                _ => false,
            };
            Ok(ctx.query(
                base,
                "find",
                Some(key_v),
                Value::Int(if found { 1 } else { 0 }),
                span,
            ))
        }
        "count" => {
            let key_v = Ctx::require_arg(args, "count", span)?;
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
            let key = ctx.value_to_key(&Ctx::require_arg(args, "erase", span)?)?;
            let old = match ctx.heap.get_mut(id) {
                Some(Object::Map(m)) => m.remove(&key),
                Some(Object::UnorderedMap(m)) => m.remove(&key),
                _ => None,
            };
            Ok(ctx.popped(
                base,
                format!("{kind}::erase"),
                Some(key.to_value()),
                old,
                span,
            ))
        }
        "insert" | "emplace" => {
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
            Ok(ctx.pushed(
                base,
                format!("{kind}::{method}"),
                None,
                Some(k.to_value()),
                v,
                span,
            ))
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
            Some(Object::Pair { first, second }) => Ok((ctx.value_to_key(first)?, second.clone())),
            _ => Err(RuntimeError::at(span, "insert expects a pair")),
        }
    } else if args.len() >= 2 {
        Ok((ctx.value_to_key(&args[0])?, args[1].clone()))
    } else {
        Err(RuntimeError::at(span, "insert needs arguments"))
    }
}
