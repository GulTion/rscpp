use super::{Engine, Flow, Frame, LValue, Result};
use crate::builtins;
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, ObjId, Object, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {
    pub(super) fn call_fn(
        &mut self,
        name: &str,
        args: &[Value],
        this: Option<Value>,
        // When Some, emit a call-site Call event before FnEnter.
        call_span: Option<Span>,
    ) -> Result<Value> {
        let func = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("undefined function `{name}`")))?;

        let parent_id = self.stack.last().map(|f| f.call_id);
        let call_id = self.next_call_id;
        self.next_call_id += 1;

        if let Some(span) = call_span {
            self.emit(Event::Call {
                name: name.to_string(),
                call_id,
                args: args.to_vec(),
                span,
            });
        }

        self.emit(Event::FnEnter {
            name: name.to_string(),
            call_id,
            parent_id,
            args: args.to_vec(),
            span: func.span,
        });

        let mut locals = HashMap::new();
        let this_for_init = this.clone();
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
            loop_stack: Vec::new(),
        });

        // Ctor member-initializer list runs before the body (`: set_(n)`).
        if !func.member_inits.is_empty() {
            if let Some(this_v) = this_for_init {
                self.apply_member_inits(&this_v, &func.member_inits)?;
            }
        }

        let flow = self.exec_block(&func.body)?;
        let ret = match flow {
            Flow::Return(v) => v,
            Flow::Next => Value::Int(0),
            Flow::Break | Flow::Continue => {
                return Err(RuntimeError::new("break/continue outside loop"));
            }
        };
        self.close_open_loops_on_return(func.span);
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

    pub(super) fn call_closure(&mut self, id: ObjId, args: &[Value], span: Span) -> Result<Value> {
        let (display, params, body, captures) = match self.heap.get(id) {
            Some(Object::Closure {
                name,
                params,
                body,
                captures,
            }) => (
                name.clone().unwrap_or_else(|| "<lambda>".into()),
                params.clone(),
                body.clone(),
                captures.clone(),
            ),
            _ => return Err(RuntimeError::at(span, "not a closure")),
        };
        if params.len() != args.len() {
            return Err(RuntimeError::at(
                span,
                format!("lambda expects {} args, got {}", params.len(), args.len()),
            ));
        }
        let parent_id = self.stack.last().map(|f| f.call_id);
        let call_id = self.next_call_id;
        self.next_call_id += 1;
        let body_span = body.span;
        self.emit(Event::Call {
            name: display.clone(),
            call_id,
            args: args.to_vec(),
            span, // call site
        });
        self.emit(Event::FnEnter {
            name: display.clone(),
            call_id,
            parent_id,
            args: args.to_vec(),
            span: body_span,
        });
        let mut locals = captures;
        for ((n, pspan), a) in params.iter().zip(args.iter()) {
            locals.insert(n.clone(), a.clone());
            self.emit(Event::VarCreate {
                name: n.clone(),
                value: a.clone(),
                span: *pspan,
            });
        }
        self.stack.push(Frame {
            name: display.clone(),
            call_id,
            parent_id,
            locals,
            loop_stack: Vec::new(),
        });
        let flow = self.exec_block(&body)?;
        let ret = match flow {
            Flow::Return(v) => v,
            Flow::Next => Value::Int(0),
            Flow::Break | Flow::Continue => {
                return Err(RuntimeError::new("break/continue outside loop"));
            }
        };
        self.close_open_loops_on_return(body_span);
        if let Some(frame) = self.stack.pop() {
            self.dealloc_owned_locals(&frame.locals, Some(&ret));
        }
        self.emit(Event::FnExit {
            name: display,
            call_id,
            parent_id,
            ret: ret.clone(),
            span: body_span,
        });
        Ok(ret)
    }

    /// Apply `: field(args)` / `: field{args}` before a ctor body.
    pub(super) fn apply_member_inits(&mut self, this: &Value, inits: &[MemberInit]) -> Result<()> {
        let Value::Object(this_id) = this else {
            return Ok(());
        };
        let cname = match self.heap.get(*this_id) {
            Some(Object::Class { name, .. }) => name.clone(),
            _ => return Ok(()),
        };
        let class = self.classes.get(&cname).cloned();
        for init in inits {
            let field_ty = class.as_ref().and_then(|c| {
                c.members.iter().find_map(|m| match m {
                    Member::Field(d) => d
                        .declarators
                        .iter()
                        .find(|dec| dec.name.name == init.name.name)
                        .map(|_| d.ty.clone()),
                    _ => None,
                })
            });
            let field_ty = field_ty.map(|t| self.resolve_type(&t).clone());
            let val = self.eval_member_init_value(field_ty.as_ref(), &init.args, init.span)?;
            if let Some(Object::Class { fields, .. }) = self.heap.get_mut(*this_id) {
                fields.insert(init.name.name.clone(), val.clone());
            }
            self.emit(Event::Write {
                slot: Slot::Field {
                    obj: *this_id,
                    field: init.name.name.clone(),
                },
                old: None,
                value: val,
                span: init.span,
            });
        }
        Ok(())
    }

    fn eval_member_init_value(
        &mut self,
        field_ty: Option<&Type>,
        args: &[Expr],
        span: Span,
    ) -> Result<Value> {
        let tname = field_ty.and_then(|ty| match ty {
            Type::Named { path, .. } => path.segments.last().map(|s| s.name.as_str()),
            Type::Const { inner, .. } => match inner.as_ref() {
                Type::Named { path, .. } => path.segments.last().map(|s| s.name.as_str()),
                _ => None,
            },
            _ => None,
        });
        // `vector<T> set_(n)` / `set_(n, fill)`
        if tname == Some("vector") && (args.len() == 1 || args.len() == 2) {
            let n = self
                .eval_expr(&args[0])?
                .as_int()
                .map_err(RuntimeError::new)? as usize;
            let fill = if args.len() == 2 {
                self.eval_expr(&args[1])?
            } else if let Some(Type::Named { args: targs, .. }) = field_ty {
                if let Some(et) = targs.first() {
                    self.default_value_for_type(et)?
                } else {
                    Value::Int(0)
                }
            } else {
                Value::Int(0)
            };
            let id = self.heap.alloc(Object::Vector(vec![fill; n]));
            self.emit_alloc(id, "vector", span);
            return Ok(Value::Object(id));
        }
        if args.len() == 1 {
            return self.eval_expr(&args[0]);
        }
        if args.is_empty() {
            if let Some(ty) = field_ty {
                return self.default_value_for_type(ty);
            }
            return Ok(Value::Int(0));
        }
        // Multi-arg non-vector: soft — evaluate first.
        self.eval_expr(&args[0])
    }

    /// Snapshot current-frame locals for `[&]`-style lambda capture (Object ids shared).
    pub(super) fn capture_locals(&self) -> HashMap<String, Value> {
        let mut caps = HashMap::new();
        if let Some(frame) = self.stack.last() {
            for (k, v) in &frame.locals {
                caps.insert(k.clone(), v.clone());
            }
        }
        // Also `this` from outer frames if missing.
        if !caps.contains_key("this") {
            for frame in self.stack.iter().rev() {
                if let Some(t) = frame.locals.get("this") {
                    caps.insert("this".into(), t.clone());
                    break;
                }
            }
        }
        caps
    }
}
