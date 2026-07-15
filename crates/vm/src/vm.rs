//! Bytecode executor.

use crate::chunk::{Op, Program};
use crate::error::VmError;
use rscpp_ast::Span;
use rscpp_runtime::{Event, Heap, MapKey, Object, Slot, Value};


type Result<T> = std::result::Result<T, VmError>;

struct Frame {
    func: usize,
    ip: usize,
    locals: Vec<Value>,
    stack_base: usize,
}

pub struct Vm {
    program: Program,
    heap: Heap,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    events: Vec<Event>,
}

impl Vm {
    pub fn new(program: Program) -> Self {
        Self {
            program,
            heap: Heap::default(),
            stack: Vec::new(),
            frames: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn run_main(&mut self) -> Result<Value> {
        let idx = self
            .program
            .main_index
            .ok_or_else(|| VmError::new("no main()"))?;
        self.call_index(idx, &[])
    }

    pub fn call(&mut self, name: &str, args: &[Value]) -> Result<Value> {
        let idx = self
            .program
            .find(name)
            .ok_or_else(|| VmError::new(format!("undefined function `{name}`")))?;
        self.call_index(idx, args)
    }

    fn call_index(&mut self, func: usize, args: &[Value]) -> Result<Value> {
        let chunk = &self.program.functions[func];
        if args.len() != chunk.arity as usize {
            return Err(VmError::new(format!(
                "arity mismatch for {}: expected {}, got {}",
                chunk.name,
                chunk.arity,
                args.len()
            )));
        }
        let mut locals = args.to_vec();
        while locals.len() < chunk.local_names.len() {
            locals.push(Value::Int(0));
        }
        // pad to at least arity slots
        while locals.len() < chunk.arity as usize {
            locals.push(Value::Int(0));
        }

        self.emit(Event::FnEnter {
            name: chunk.name.clone(),
            args: args.to_vec(),
            span: Span::new(0, 0),
        });

        self.frames.push(Frame {
            func,
            ip: 0,
            locals,
            stack_base: self.stack.len(),
        });

        self.run_loop()
    }

    fn emit(&mut self, e: Event) {
        self.events.push(e);
    }

    fn run_loop(&mut self) -> Result<Value> {
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
                Op::Call { func: name_idx, argc } => {
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
                    self.emit(Event::FnEnter {
                        name: chunk.name.clone(),
                        args,
                        span,
                    });
                    self.frames.push(Frame {
                        func: callee,
                        ip: 0,
                        locals,
                        stack_base: self.stack.len(),
                    });
                }
                Op::Return => {
                    let ret = self.pop().unwrap_or(Value::Int(0));
                    let finished = self.frames.pop().unwrap();
                    let finished_name = self.program.functions[finished.func].name.clone();
                    self.stack.truncate(finished.stack_base);
                    self.emit(Event::FnExit {
                        name: finished_name,
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
                    self.emit(Event::Alloc {
                        id,
                        kind: name,
                        span,
                    });
                    // Grow locals naming for VarCreate on decl — handled by StoreLocal after
                    self.stack.push(Value::Object(id));
                }
                Op::MakePair => {
                    let second = self.pop()?;
                    let first = self.pop()?;
                    let id = self.heap.alloc(Object::Pair { first, second });
                    self.emit(Event::Alloc {
                        id,
                        kind: "pair".into(),
                        span,
                    });
                    self.stack.push(Value::Object(id));
                }
            }
        }
    }

    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| VmError::new("stack underflow"))
    }

    fn index_get(&self, base: Value, idx: Value, span: Span) -> Result<Value> {
        let Value::Object(id) = base else {
            return Err(VmError::at(span, "index on non-object"));
        };
        match self.heap.get(id) {
            Some(Object::Vector(e)) => {
                let i = idx.as_int().map_err(VmError::new)? as usize;
                e.get(i)
                    .cloned()
                    .ok_or_else(|| VmError::at(span, "oob"))
            }
            Some(Object::Map(m)) => {
                let k = map_key(&idx)?;
                Ok(m.get(&k).cloned().unwrap_or(Value::Int(0)))
            }
            Some(Object::UnorderedMap(m)) => {
                let k = map_key(&idx)?;
                Ok(m.get(&k).cloned().unwrap_or(Value::Int(0)))
            }
            _ => Err(VmError::at(span, "not indexable")),
        }
    }

    fn index_set(&mut self, base: Value, idx: Value, val: Value, span: Span) -> Result<()> {
        let Value::Object(id) = base else {
            return Err(VmError::at(span, "index set on non-object"));
        };
        match self.heap.get_mut(id) {
            Some(Object::Vector(e)) => {
                let i = idx.as_int().map_err(VmError::new)? as usize;
                if i >= e.len() {
                    return Err(VmError::at(span, "oob"));
                }
                let old = e[i].clone();
                e[i] = val.clone();
                self.emit(Event::Write {
                    slot: Slot::Index { obj: id, index: i },
                    old: Some(old),
                    value: val,
                    span,
                });
            }
            Some(Object::Map(m)) => {
                let k = map_key(&idx)?;
                let old = m.insert(k.clone(), val.clone());
                self.emit(Event::Write {
                    slot: Slot::MapEntry {
                        obj: id,
                        key: k.to_string(),
                    },
                    old,
                    value: val,
                    span,
                });
            }
            Some(Object::UnorderedMap(m)) => {
                let k = map_key(&idx)?;
                let old = m.insert(k.clone(), val.clone());
                self.emit(Event::Write {
                    slot: Slot::MapEntry {
                        obj: id,
                        key: k.to_string(),
                    },
                    old,
                    value: val,
                    span,
                });
            }
            _ => return Err(VmError::at(span, "not indexable")),
        }
        Ok(())
    }

    fn call_method(
        &mut self,
        base: Value,
        method: &str,
        args: &[Value],
        span: Span,
    ) -> Result<Value> {
        let Value::Object(id) = base.clone() else {
            return Err(VmError::at(span, "method on non-object"));
        };
        let kind = self
            .heap
            .get(id)
            .map(|o| o.kind_name())
            .ok_or_else(|| VmError::at(span, "dangling object"))?;

        let mut ctx = rscpp_runtime::stl::Ctx {
            heap: &mut self.heap,
            events: &mut self.events,
        };
        rscpp_runtime::stl::call_method(&mut ctx, id, base, kind, method, args, span)
            .map_err(VmError::from)
    }

    pub fn make_vector(&mut self, elems: Vec<Value>) -> Value {
        let id = self.heap.alloc(Object::Vector(elems));
        Value::Object(id)
    }

    pub fn vector_as_ints(&self, v: &Value) -> Result<Vec<i64>> {
        let Value::Object(id) = v else {
            return Err(VmError::new("expected vector"));
        };
        match self.heap.get(*id) {
            Some(Object::Vector(e)) => e
                .iter()
                .map(|x| x.as_int().map_err(VmError::new))
                .collect(),
            _ => Err(VmError::new("expected vector")),
        }
    }
}

fn map_key(v: &Value) -> Result<MapKey> {
    match v {
        Value::Int(i) => Ok(MapKey::Int(*i)),
        Value::Bool(b) => Ok(MapKey::Bool(*b)),
        Value::Char(c) => Ok(MapKey::Char(*c)),
        _ => Err(VmError::new("bad map key")),
    }
}

/// Parse, compile, run `main`.
pub fn run_main(src: &str) -> Result<(Value, Vec<Event>)> {
    let tu = rscpp_parser::parse(src)?;
    let program = crate::compile::compile(&tu)?;
    let mut vm = Vm::new(program);
    let v = vm.run_main()?;
    Ok((v, vm.take_events()))
}

/// Parse, compile, call a named function.
pub fn call(src: &str, name: &str, args: &[Value]) -> Result<(Value, Vec<Event>)> {
    let tu = rscpp_parser::parse(src)?;
    let program = crate::compile::compile(&tu)?;
    let mut vm = Vm::new(program);
    let v = vm.call(name, args)?;
    Ok((v, vm.take_events()))
}
