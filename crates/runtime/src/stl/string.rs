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
    if !matches!(ctx.heap.get(id), Some(Object::String(_))) {
        return Err(RuntimeError::at(span, "not a string"));
    }
    match method {
        "size" | "length" => Ok(ctx.length_alias(id, base, method, span)),
        "empty" => Ok(ctx.empty(id, base, span)),
        "clear" => Ok(ctx.clear(id, base, "string::clear", span)),
        "begin" | "end" | "cbegin" | "cend" => Ok(Value::Int(0)),
        "front" | "back" => {
            let Some(Object::String(s)) = ctx.heap.get(id) else {
                return Err(RuntimeError::at(span, "not a string"));
            };
            let ch = if method == "front" {
                s.chars().next()
            } else {
                s.chars().next_back()
            }
            .ok_or_else(|| RuntimeError::at(span, format!("{method} on empty string")))?;
            Ok(ctx.query(base, method, None, Value::Char(ch), span))
        }
        "push_back" | "emplace_back" => {
            let ch = arg_as_char(args, method, span)?;
            if let Some(Object::String(s)) = ctx.heap.get_mut(id) {
                s.push(ch);
            }
            Ok(ctx.pushed(base, "string::push_back", None, None, Value::Char(ch), span))
        }
        "append" => {
            // `s.append(t)` / `s.append(c)` — returns `s` for chaining.
            let extra = stringish_arg(ctx, args, span)?;
            if let Some(Object::String(s)) = ctx.heap.get_mut(id) {
                s.push_str(&extra);
            }
            ctx.modify(
                base.clone(),
                "string::append",
                None,
                None,
                None,
                Some(Value::Str(extra)),
                span,
            );
            Ok(base)
        }
        "substr" => {
            let start = args
                .first()
                .ok_or_else(|| RuntimeError::at(span, "substr needs start"))?
                .as_int()
                .map_err(RuntimeError::new)? as usize;
            let Some(Object::String(s)) = ctx.heap.get(id).cloned() else {
                return Err(RuntimeError::at(span, "not a string"));
            };
            let chars: Vec<char> = s.chars().collect();
            let slice = if args.len() >= 2 {
                let len = args[1].as_int().map_err(RuntimeError::new)? as usize;
                chars.get(start..start.saturating_add(len).min(chars.len()))
            } else {
                chars.get(start..)
            }
            .map(|c| c.iter().collect::<String>())
            .unwrap_or_default();
            let nid = ctx.heap.alloc(Object::String(slice.clone()));
            let (size, elems, entries) = Object::String(slice).alloc_snapshot();
            ctx.emit(crate::event::Event::Alloc {
                call_id: ctx.call_id,
                id: nid,
                kind: "string".into(),
                size,
                elems,
                entries,
                span,
            });
            Ok(Value::Object(nid))
        }
        "find" => {
            let needle = stringish_arg(ctx, args, span)?;
            let Some(Object::String(s)) = ctx.heap.get(id) else {
                return Err(RuntimeError::at(span, "not a string"));
            };
            let pos = s.find(&needle).map(|i| i as i64).unwrap_or(-1);
            // C++ npos is usually size_t(-1); we use -1 as int.
            Ok(ctx.query(base, "find", Some(Value::Str(needle)), Value::Int(pos), span))
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown string method `{method}`"),
        )),
    }
}

fn arg_as_char(args: &[Value], method: &str, span: Span) -> Result<char> {
    match args.first() {
        Some(Value::Char(c)) => Ok(*c),
        Some(Value::Int(n)) => Ok(char::from_u32((*n as u8) as u32).unwrap_or('\0')),
        Some(v) => Err(RuntimeError::at(
            span,
            format!("string::{method} expects char, got {v}"),
        )),
        None => Err(RuntimeError::at(span, format!("{method} needs an argument"))),
    }
}

fn stringish_arg(ctx: &Ctx<'_>, args: &[Value], span: Span) -> Result<String> {
    match args.first() {
        Some(Value::Object(id)) => match ctx.heap.get(*id) {
            Some(Object::String(s)) => Ok(s.clone()),
            _ => Err(RuntimeError::at(span, "expected string argument")),
        },
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(Value::Char(c)) => Ok(c.to_string()),
        Some(Value::Int(n)) => Ok(n.to_string()),
        Some(v) => Err(RuntimeError::at(span, format!("expected string-ish, got {v}"))),
        None => Err(RuntimeError::at(span, "needs an argument")),
    }
}
