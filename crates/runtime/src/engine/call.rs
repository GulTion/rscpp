use super::{Engine, Flow, Frame, LValue, Result};
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::builtins;
use crate::value::{Address, Heap, MapKey, Object, ObjId, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {

    pub(super) fn call_fn(&mut self, name: &str, args: &[Value], this: Option<Value>) -> Result<Value> {
        let func = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("undefined function `{name}`")))?;

        let parent_id = self.stack.last().map(|f| f.call_id);
        let call_id = self.next_call_id;
        self.next_call_id += 1;

        self.emit(Event::FnEnter {
            name: name.to_string(),
            call_id,
            parent_id,
            args: args.to_vec(),
            span: func.span,
        });

        let mut locals = HashMap::new();
        if let Some(t) = this {
            locals.insert("this".into(), t);
        }
        if func.params.len() != args.len() {
            return Err(RuntimeError::at(
                func.span,
                format!(
                    "function `{name}` expects {} args, got {}",
                    func.params.len(),
                    args.len()
                ),
            ));
        }
        for (p, a) in func.params.iter().zip(args.iter()) {
            if let Some(n) = &p.name {
                let bound = self.bind_param_value(&p.ty, a, &n.name, n.span)?;
                locals.insert(n.name.clone(), bound.clone());
                self.emit(Event::VarCreate {
                    call_id: self.current_call_id(),
                    name: n.name.clone(),
                    value: bound,
                    span: n.span,
                });
            }
        }

        self.stack.push(Frame {
            name: name.to_string(),
            call_id,
            parent_id,
            locals,
        });
        let flow = self.exec_block(&func.body)?;
        let ret = match flow {
            Flow::Return(v) => v,
            Flow::Next => Value::Int(0),
            Flow::Break | Flow::Continue => {
                return Err(RuntimeError::new("break/continue outside loop"));
            }
        };
        if let Some(frame) = self.stack.pop() {
            self.dealloc_owned_locals(&frame.locals, Some(&ret));
        }

        self.emit(Event::FnExit {
            name: name.to_string(),
            call_id,
            parent_id,
            ret: ret.clone(),
            span: func.span,
        });
        Ok(ret)
    }

    /// Resolve `foo` → `Class::foo` when called from a method (LeetCode-style unqualified calls).
    pub(super) fn resolve_fn_call(&self, name: &str) -> Result<(String, Option<Value>)> {
        if self.functions.contains_key(name) {
            return Ok((name.to_string(), None));
        }
        // From `this` object's class name
        if let Ok(this_v) = self.lookup_raw("this") {
            if let Value::Object(id) = &this_v {
                if let Some(Object::Class { name: cname, .. }) = self.heap.get(*id) {
                    let q = format!("{cname}::{name}");
                    if self.functions.contains_key(&q) {
                        return Ok((q, Some(this_v)));
                    }
                }
            }
        }
        // From current frame `Solution::countComponents` → try `Solution::name`
        if let Some(frame) = self.stack.last() {
            if let Some((cls, _)) = frame.name.split_once("::") {
                let q = format!("{cls}::{name}");
                if self.functions.contains_key(&q) {
                    let this = self.lookup_raw("this").ok();
                    return Ok((q, this));
                }
            }
        }
        Err(RuntimeError::new(format!("undefined function `{name}`")))
    }
}
