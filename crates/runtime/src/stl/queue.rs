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
            Some(Object::Queue(q)) => Ok(Value::Int(q.len() as i64)),
            _ => Ok(Value::Int(0)),
        },
        "empty" => match ctx.heap.get(id) {
            Some(Object::Queue(q)) => Ok(Value::Bool(q.is_empty())),
            _ => Ok(Value::Bool(true)),
        },
        "front" => match ctx.heap.get(id) {
            Some(Object::Queue(q)) => q
                .front()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "front on empty queue")),
            _ => Err(RuntimeError::at(span, "not a queue")),
        },
        "back" => match ctx.heap.get(id) {
            Some(Object::Queue(q)) => q
                .back()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "back on empty queue")),
            _ => Err(RuntimeError::at(span, "not a queue")),
        },
        "push" => {
            let v = args
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::at(span, "push needs a value"))?;
            if let Some(Object::Queue(q)) = ctx.heap.get_mut(id) {
                q.push_back(v.clone());
            }
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: "queue::push".into(),
                index: None,
                old: None,
                value: Some(v),
                span,
            });
            Ok(Value::Void)
        }
        "pop" => {
            let old = if let Some(Object::Queue(q)) = ctx.heap.get_mut(id) {
                q.pop_front()
            } else {
                None
            };
            ctx.emit(Event::ContainerMod {
                container: base,
                kind: "queue::pop".into(),
                index: None,
                old,
                value: None,
                span,
            });
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::at(
            span,
            format!("unknown queue method `{method}`"),
        )),
    }
}
