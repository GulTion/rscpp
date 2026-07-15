use super::{Ctx, Result};
use crate::error::RuntimeError;
use crate::event::Event;
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
        "size" => match ctx.heap.get(id) {
            Some(Object::Stack(s)) => Ok(Value::Int(s.len() as i64)),
            _ => Ok(Value::Int(0)),
        },
        "empty" => match ctx.heap.get(id) {
            Some(Object::Stack(s)) => Ok(Value::Bool(s.is_empty())),
            _ => Ok(Value::Bool(true)),
        },
        "top" => match ctx.heap.get(id) {
            Some(Object::Stack(s)) => s
                .last()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "top on empty stack")),
            _ => Err(RuntimeError::at(span, "not a stack")),
        },
        "push" => {
            let v = args
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "push needs a value"))?;
            if let Some(Object::Stack(s)) = ctx.heap.get_mut(id) {
                s.push(v.clone());
            }
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: "stack::push".into(),
                index: None,
                old: None,
                value: Some(v),
                span,
            });
            Ok(Value::Void)
        }
        "pop" => {
            let old = if let Some(Object::Stack(s)) = ctx.heap.get_mut(id) {
                s.pop()
            } else {
                None
            };
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: "stack::pop".into(),
                index: None,
                old,
                value: None,
                span,
            });
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown stack method `{method}`"),
        )),
    }
}
