use super::Ctx;
use crate::error::RuntimeError;
use crate::value::{ObjId, Object, Value};
use rscpp_ast::Span;

type Result<T> = std::result::Result<T, RuntimeError>;

pub fn call(
    ctx: &mut Ctx<'_>,
    id: ObjId,
    base: Value,
    method: &str,
    args: &[Value],
    span: Span,
) -> Result<Value> {
    let n = match ctx.heap.get(id) {
        Some(Object::Bitset { bits }) => bits.len(),
        _ => return Err(RuntimeError::at(span, "not a bitset")),
    };
    match method {
        "size" => Ok(ctx.query(base, "size", None, Value::Int(n as i64), span)),
        "count" => {
            let c = match ctx.heap.get(id) {
                Some(Object::Bitset { bits }) => bits.iter().filter(|b| **b).count() as i64,
                _ => 0,
            };
            Ok(ctx.query(base, "count", None, Value::Int(c), span))
        }
        "any" => {
            let v = match ctx.heap.get(id) {
                Some(Object::Bitset { bits }) => bits.iter().any(|b| *b),
                _ => false,
            };
            Ok(ctx.query(base, "any", None, Value::Bool(v), span))
        }
        "none" => {
            let v = match ctx.heap.get(id) {
                Some(Object::Bitset { bits }) => bits.iter().all(|b| !*b),
                _ => true,
            };
            Ok(ctx.query(base, "none", None, Value::Bool(v), span))
        }
        "all" => {
            let v = match ctx.heap.get(id) {
                Some(Object::Bitset { bits }) => !bits.is_empty() && bits.iter().all(|b| *b),
                _ => false,
            };
            Ok(ctx.query(base, "all", None, Value::Bool(v), span))
        }
        "test" => {
            if args.len() != 1 {
                return Err(RuntimeError::at(span, "bitset::test expects 1 arg"));
            }
            let i = args[0].as_int().map_err(RuntimeError::new)? as usize;
            let bit = match ctx.heap.get(id) {
                Some(Object::Bitset { bits }) => bits
                    .get(i)
                    .copied()
                    .ok_or_else(|| RuntimeError::at(span, "bitset::test out of range"))?,
                _ => false,
            };
            Ok(ctx.query(base, "test", Some(args[0].clone()), Value::Bool(bit), span))
        }
        "set" => {
            match args {
                [] => {
                    if let Some(Object::Bitset { bits }) = ctx.heap.get_mut(id) {
                        for b in bits.iter_mut() {
                            *b = true;
                        }
                    }
                }
                [pos] => {
                    let i = pos.as_int().map_err(RuntimeError::new)? as usize;
                    if let Some(Object::Bitset { bits }) = ctx.heap.get_mut(id) {
                        bits.get_mut(i)
                            .map(|b| *b = true)
                            .ok_or_else(|| RuntimeError::at(span, "bitset::set out of range"))?;
                    }
                }
                [pos, val] => {
                    let i = pos.as_int().map_err(RuntimeError::new)? as usize;
                    let bit = val.as_bool().map_err(RuntimeError::new)?;
                    if let Some(Object::Bitset { bits }) = ctx.heap.get_mut(id) {
                        bits.get_mut(i)
                            .map(|b| *b = bit)
                            .ok_or_else(|| RuntimeError::at(span, "bitset::set out of range"))?;
                    }
                }
                _ => return Err(RuntimeError::at(span, "bitset::set expects 0..=2 args")),
            }
            ctx.modify(base.clone(), "set", None, None, None, None, span);
            Ok(base)
        }
        "reset" => {
            match args {
                [] => {
                    if let Some(Object::Bitset { bits }) = ctx.heap.get_mut(id) {
                        for b in bits.iter_mut() {
                            *b = false;
                        }
                    }
                }
                [pos] => {
                    let i = pos.as_int().map_err(RuntimeError::new)? as usize;
                    if let Some(Object::Bitset { bits }) = ctx.heap.get_mut(id) {
                        bits.get_mut(i)
                            .map(|b| *b = false)
                            .ok_or_else(|| RuntimeError::at(span, "bitset::reset out of range"))?;
                    }
                }
                _ => return Err(RuntimeError::at(span, "bitset::reset expects 0..=1 args")),
            }
            ctx.modify(base.clone(), "reset", None, None, None, None, span);
            Ok(base)
        }
        "flip" => {
            match args {
                [] => {
                    if let Some(Object::Bitset { bits }) = ctx.heap.get_mut(id) {
                        for b in bits.iter_mut() {
                            *b = !*b;
                        }
                    }
                }
                [pos] => {
                    let i = pos.as_int().map_err(RuntimeError::new)? as usize;
                    if let Some(Object::Bitset { bits }) = ctx.heap.get_mut(id) {
                        bits.get_mut(i)
                            .map(|b| *b = !*b)
                            .ok_or_else(|| RuntimeError::at(span, "bitset::flip out of range"))?;
                    }
                }
                _ => return Err(RuntimeError::at(span, "bitset::flip expects 0..=1 args")),
            }
            ctx.modify(base.clone(), "flip", None, None, None, None, span);
            Ok(base)
        }
        "to_string" | "to_ulong" | "to_ullong" => {
            let bits = match ctx.heap.get(id) {
                Some(Object::Bitset { bits }) => bits.clone(),
                _ => vec![],
            };
            if method == "to_string" {
                let s: String = bits
                    .iter()
                    .rev()
                    .map(|b| if *b { '1' } else { '0' })
                    .collect();
                // Allocate via returning Str — caller may prefer heap string; use Str for simplicity.
                Ok(ctx.query(base, "to_string", None, Value::Str(s), span))
            } else {
                let mut v = 0u64;
                for (i, b) in bits.iter().enumerate().take(64) {
                    if *b {
                        v |= 1u64 << i;
                    }
                }
                Ok(ctx.query(base, method, None, Value::Int(v as i64), span))
            }
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown bitset method `{method}`"),
        )),
    }
}
