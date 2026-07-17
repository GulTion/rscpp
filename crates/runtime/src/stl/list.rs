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
    let Some(Object::List(elems)) = ctx.heap.get(id).cloned() else {
        return Err(RuntimeError::at(span, "not a list"));
    };
    match method {
        "begin" | "cbegin" => Ok(Value::Int(0)),
        "end" | "cend" => Ok(Value::Int(elems.len() as i64)),
        "size" => Ok(ctx.size(id, base, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
        "clear" => Ok(ctx.clear(id, base, "clear", span)),
        "front" => {
            let v = elems
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "front on empty list"))?;
            Ok(ctx.query(base, "front", Some(Value::Int(0)), v, span))
        }
        "back" => {
            let idx = elems.len().saturating_sub(1) as i64;
            let v = elems
                .last()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "back on empty list"))?;
            Ok(ctx.query(base, "back", Some(Value::Int(idx)), v, span))
        }
        "push_back" | "emplace_back" => {
            let v = Ctx::require_arg(args, method, span)?;
            if let Some(Object::List(e)) = ctx.heap.get_mut(id) {
                e.push(v.clone());
            }
            Ok(ctx.pushed(base, method, None, None, v, span))
        }
        "push_front" | "emplace_front" => {
            let v = Ctx::require_arg(args, method, span)?;
            if let Some(Object::List(e)) = ctx.heap.get_mut(id) {
                e.insert(0, v.clone());
            }
            Ok(ctx.pushed(base, method, Some(0), None, v, span))
        }
        "pop_back" => {
            let old = if let Some(Object::List(e)) = ctx.heap.get_mut(id) {
                e.pop()
            } else {
                None
            };
            Ok(ctx.popped(base, "pop_back", None, old, span))
        }
        "pop_front" => {
            let old = if let Some(Object::List(e)) = ctx.heap.get_mut(id) {
                if e.is_empty() {
                    None
                } else {
                    Some(e.remove(0))
                }
            } else {
                None
            };
            Ok(ctx.popped(base, "pop_front", None, old, span))
        }
        "insert" | "emplace" => {
            if args.len() < 2 {
                return Err(RuntimeError::at(span, format!("{method} needs pos, value")));
            }
            let pos = seq_index(id, &args[0], span)?;
            let v = args[1].clone();
            if let Some(Object::List(e)) = ctx.heap.get_mut(id) {
                if pos > e.len() {
                    return Err(RuntimeError::at(span, "insert past end"));
                }
                e.insert(pos, v.clone());
            }
            Ok(ctx.pushed(base, method, Some(pos), None, v, span))
        }
        "erase" => {
            if args.is_empty() {
                return Err(RuntimeError::at(span, "erase needs position"));
            }
            let first = seq_index(id, &args[0], span)?;
            if let Some(Object::List(e)) = ctx.heap.get_mut(id) {
                if first >= e.len() {
                    return Err(RuntimeError::at(span, "erase out of range"));
                }
                e.remove(first);
            }
            ctx.modify(base, "erase", Some(first), None, None, None, span);
            Ok(Value::Int(first as i64))
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown list method `{method}`"),
        )),
    }
}
