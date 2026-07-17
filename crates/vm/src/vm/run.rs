use super::{Frame, Result, Vm};
use crate::chunk::{Op, Program};
use crate::error::VmError;
use rscpp_ast::Span;
use rscpp_runtime::{Event, Heap, MapKey, Object, Slot, Value};

impl Vm {
    pub(super) fn run_loop(&mut self) -> Result<Value> {
        loop {
            let frame_i = self.frames.len() - 1;
            let func = self.frames[frame_i].func;
            let ip = self.frames[frame_i].ip;
            let chunk = &self.program.functions[func];
            if ip >= chunk.ops.len() {
                return Err(VmError::new("ip out of range (missing return?)"));
            }
            let op = chunk.ops[ip].clone();
            let span = chunk.spans[ip];
            self.frames[frame_i].ip = ip + 1;

            match op {
                Op::Step => {
                    self.emit(Event::Step { span });
                }
                Op::LoadConst(i) => {
                    let v = self.program.functions[func].constants[i as usize].clone();
                    self.stack.push(v);
                }
                Op::LoadLocal(i) => {
                    let v = self.frames[frame_i].locals[i as usize].clone();
                    self.stack.push(v);
                }
                Op::StoreLocal(i) => {
                    let v = self.pop()?;
                    let name = self.program.functions[func]
                        .local_names
                        .get(i as usize)
                        .cloned()
                        .unwrap_or_else(|| format!("local{i}"));
                    let old = self.frames[frame_i].locals[i as usize].clone();
                    self.frames[frame_i].locals[i as usize] = v.clone();
                    self.emit(Event::VarAssign {
                        name: name.clone(),
                        old: Some(old.clone()),
                        value: v.clone(),
                        span,
                    });
                    self.emit(Event::Write {
                        slot: Slot::Local { name },
                        old: Some(old),
                        value: v,
                        span,
                    });
                }
                Op::Pop => {
                    let _ = self.pop()?;
                }
                Op::Dup => {
                    let v = self
                        .stack
                        .last()
                        .cloned()
                        .ok_or_else(|| VmError::at(span, "stack underflow"))?;
                    self.stack.push(v);
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => {
                    let b = self.pop()?.as_int().map_err(VmError::new)?;
                    let a = self.pop()?.as_int().map_err(VmError::new)?;
                    let n = match op {
                        Op::Add => a + b,
                        Op::Sub => a - b,
                        Op::Mul => a * b,
                        Op::Div => {
                            if b == 0 {
                                return Err(VmError::at(span, "division by zero"));
                            }
                            a / b
                        }
                        Op::Rem => {
                            if b == 0 {
                                return Err(VmError::at(span, "division by zero"));
                            }
                            a % b
                        }
                        _ => unreachable!(),
                    };
                    self.stack.push(Value::Int(n));
                }
                Op::Neg => {
                    let a = self.pop()?.as_int().map_err(VmError::new)?;
                    self.stack.push(Value::Int(-a));
                }
                Op::Not => {
                    let a = self.pop()?;
                    self.stack
                        .push(Value::Bool(!a.as_bool().map_err(VmError::new)?));
                }
                Op::CmpLt | Op::CmpLe | Op::CmpGt | Op::CmpGe | Op::CmpEq | Op::CmpNe => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let ai = a.as_int().map_err(VmError::new)?;
                    let bi = b.as_int().map_err(VmError::new)?;
                    let result = match op {
                        Op::CmpLt => ai < bi,
                        Op::CmpLe => ai <= bi,
                        Op::CmpGt => ai > bi,
                        Op::CmpGe => ai >= bi,
                        Op::CmpEq => ai == bi,
                        Op::CmpNe => ai != bi,
                        _ => unreachable!(),
                    };
                    self.emit(Event::Compare {
                        op: format!("{op:?}"),
                        left: a,
                        right: b,
                        result,
                        span,
                    });
                    self.stack.push(Value::Bool(result));
                }
                Op::Jump(t) => {
                    self.frames[frame_i].ip = t as usize;
                }
                Op::JumpIfFalse(t) => {
                    let v = self.pop()?;
                    if !v.as_bool().map_err(VmError::new)? {
                        self.frames[frame_i].ip = t as usize;
                    }
                }
                Op::Call {
                    func: name_idx,
                    argc,
                } => {
                    let name = self.program.functions[func].string_pool[name_idx as usize].clone();
                    let mut args = Vec::new();
                    for _ in 0..argc {
                        args.push(self.pop()?);
                    }
                    args.reverse();
                    let callee = self
                        .program
                        .find(&name)
                        .ok_or_else(|| VmError::at(span, format!("undefined `{name}`")))?;
                    let chunk = &self.program.functions[callee];
                    if args.len() != chunk.arity as usize {
                        return Err(VmError::at(
                            span,
                            format!(
                                "arity mismatch for {}: expected {}, got {}",
                                chunk.name,
                                chunk.arity,
                                args.len()
                            ),
                        ));
                    }
                    let mut locals = args.clone();
                    while locals.len() < chunk.local_names.len().max(chunk.arity as usize) {
                        locals.push(Value::Int(0));
                    }
                    let parent_id = self.frames.last().map(|f| f.call_id);
                    let call_id = self.next_call_id;
                    self.next_call_id += 1;
                    self.emit(Event::FnEnter {
                        name: chunk.name.clone(),
                        call_id,
                        parent_id,
                        args,
                        span,
                    });
                    self.frames.push(Frame {
                        func: callee,
                        ip: 0,
                        locals,
                        stack_base: self.stack.len(),
                        call_id,
                        parent_id,
                    });
                }
                Op::Return => {
                    let ret = self.pop().unwrap_or(Value::Int(0));
                    let finished = self.frames.pop().unwrap();
                    let finished_name = self.program.functions[finished.func].name.clone();
                    self.stack.truncate(finished.stack_base);
                    self.emit(Event::FnExit {
                        name: finished_name,
                        call_id: finished.call_id,
                        parent_id: finished.parent_id,
                        ret: ret.clone(),
                        span,
                    });
                    if self.frames.is_empty() {
                        return Ok(ret);
                    }
                    self.stack.push(ret);
                }
                Op::CallMethod { name, argc } => {
                    let method = self.program.functions[func].string_pool[name as usize].clone();
                    let mut args = Vec::new();
                    for _ in 0..argc {
                        args.push(self.pop()?);
                    }
                    args.reverse();
                    let base = self.pop()?;
                    let ret = self.call_method(base, &method, &args, span)?;
                    self.stack.push(ret);
                }
                Op::IndexGet => {
                    let idx = self.pop()?;
                    let base = self.pop()?;
                    let v = self.index_get(base, idx, span)?;
                    self.stack.push(v);
                }
                Op::IndexSet => {
                    let val = self.pop()?;
                    let idx = self.pop()?;
                    let base = self.pop()?;
                    self.index_set(base, idx, val.clone(), span)?;
                    self.stack.push(val);
                }
                Op::NewEmpty { type_name } => {
                    let name = self.program.functions[func].string_pool[type_name as usize].clone();
                    let obj = Object::empty_named(&name).ok_or_else(|| {
                        VmError::at(span, format!("unknown type `{name}` for NewEmpty"))
                    })?;
                    let id = self.heap.alloc(obj);
                    self.emit_alloc(id, name, span);
                    // Grow locals naming for VarCreate on decl — handled by StoreLocal after
                    self.stack.push(Value::Object(id));
                }
                Op::MakePair => {
                    let second = self.pop()?;
                    let first = self.pop()?;
                    let id = self.heap.alloc(Object::Pair { first, second });
                    self.emit_alloc(id, "pair", span);
                    self.stack.push(Value::Object(id));
                }
            }
        }
    }
}
