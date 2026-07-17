use super::{Ctx, Result};
use crate::error::RuntimeError;
use crate::value::{Address, ObjId, Object, Value};
use rscpp_ast::Span;

fn seq_index(id: ObjId, pos: &Value, span: Span) -> Result<usize> {
    match pos {
        Value::Int(i) if *i >= 0 => Ok(*i as usize),
        Value::Ptr(Address::Index { obj, index }) if *obj == id => Ok(*index),
        _ => Err(RuntimeError::at(span, "insert/erase position")),
    }
}

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
        "begin" | "cbegin" => Ok(Value::Int(0)),
        "end" | "cend" => Ok(Value::Int(elems.len() as i64)),
        "rbegin" | "crbegin" => Ok(Value::Int(0)),
        "rend" | "crend" => Ok(Value::Int(elems.len() as i64)),
        "size" => Ok(ctx.size(id, base, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
        "push_back" | "emplace_back" => {
            let v = Ctx::require_arg(args, method, span)?;
            let idx = if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                e.push(v.clone());
                e.len() - 1
            } else {
                0
            };
            Ok(ctx.pushed(base, method, Some(idx), None, v, span))
        }
        "pop_back" => {
            let old = if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                e.pop()
            } else {
                None
            };
            Ok(ctx.popped(base, "pop_back", None, old, span))
        }
        "front" => {
            let v = elems
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "front on empty vector"))?;
            Ok(ctx.query(base, "front", Some(Value::Int(0)), v, span))
        }
        "back" => {
            let idx = elems.len().saturating_sub(1) as i64;
            let v = elems
                .last()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "back on empty vector"))?;
            Ok(ctx.query(base, "back", Some(Value::Int(idx)), v, span))
        }
        "clear" => Ok(ctx.clear(id, base, "clear", span)),
        "at" => {
            let idx = Ctx::require_arg(args, method, span)?
                .as_int()
                .map_err(RuntimeError::new)? as usize;
            let v = elems.get(idx).cloned().ok_or_else(|| {
                RuntimeError::at(span, format!("vector::at index {idx} out of range"))
            })?;
            Ok(ctx.query(base, "at", Some(Value::Int(idx as i64)), v, span))
        }
        "reserve" => {
            // ponytail: capacity tracking unused; growth is free
            let _n = Ctx::require_arg(args, method, span)?;
            ctx.modify(base, "reserve", None, None, None, None, span);
            Ok(Value::Void)
        }
        "capacity" => {
            let n = elems.len() as i64;
            Ok(ctx.query(base, "capacity", None, Value::Int(n), span))
        }
        "resize" => {
            let n = Ctx::require_arg(args, method, span)?
                .as_int()
                .map_err(RuntimeError::new)?;
            if n < 0 {
                return Err(RuntimeError::at(span, "vector::resize negative"));
            }
            let n = n as usize;
            let fill = if args.len() >= 2 {
                args[1].clone()
            } else {
                Value::Int(0)
            };
            if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                if n < e.len() {
                    e.truncate(n);
                } else {
                    e.resize(n, fill);
                }
            }
            ctx.modify(base, "resize", Some(n), None, None, None, span);
            Ok(Value::Void)
        }
        "insert" | "emplace" => {
            if args.len() < 2 {
                return Err(RuntimeError::at(span, format!("{method} needs pos, value")));
            }
            let pos = seq_index(id, &args[0], span)?;
            let v = args[1].clone();
            let len = elems.len();
            if pos > len {
                return Err(RuntimeError::at(span, "insert past end"));
            }
            if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                e.insert(pos, v.clone());
            }
            Ok(ctx.pushed(base, method, Some(pos), None, v, span))
        }
        "erase" => {
            if args.is_empty() {
                return Err(RuntimeError::at(span, "erase needs position"));
            }
            let first = seq_index(id, &args[0], span)?;
            if args.len() >= 2 {
                let last = seq_index(id, &args[1], span)?;
                if last < first {
                    return Err(RuntimeError::at(span, "erase invalid range"));
                }
                if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                    if last > e.len() || first > e.len() {
                        return Err(RuntimeError::at(span, "erase out of range"));
                    }
                    e.drain(first..last);
                }
                ctx.modify(base, "erase", Some(first), None, None, None, span);
            } else if let Some(Object::Vector(e)) = ctx.heap.get_mut(id) {
                if first >= e.len() {
                    return Err(RuntimeError::at(span, "erase out of range"));
                }
                e.remove(first);
                ctx.modify(base, "erase", Some(first), None, None, None, span);
            }
            // Return stub iterator = index (LeetCode erase-remove)
            Ok(Value::Int(first as i64))
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown vector method `{method}`"),
        )),
    }
}
