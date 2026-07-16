//! Tree-walking interpreter.

use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, Object, ObjId, Value};
use rscpp_ast::*;
use rscpp_parser::parse;
use rscpp_sema::analyze;
use std::collections::HashMap;

type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug)]
struct Frame {
    #[allow(dead_code)]
    name: String,
    /// Activation id for this frame (matches `FnEnter.call_id`).
    call_id: u64,
    #[allow(dead_code)]
    parent_id: Option<u64>,
    locals: HashMap<String, Value>,
}

#[derive(Debug)]
enum Flow {
    Next,
    Return(Value),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
enum LValue {
    Name(String),
    Index { obj: ObjId, index: usize },
    MapEntry { obj: ObjId, key: MapKey },
    Field { obj: ObjId, field: String },
}

/// Parsed program + heap + call stack + event log.
pub struct Engine {
    functions: HashMap<String, FunctionDef>,
    classes: HashMap<String, ClassDef>,
    globals: HashMap<String, Value>,
    heap: Heap,
    /// Mapped-type for `map`/`unordered_map` `operator[]` default-insert (`map<K,V>` → `V`).
    map_value_tys: HashMap<ObjId, Type>,
    stack: Vec<Frame>,
    /// Next `call_id` to assign on `FnEnter`.
    next_call_id: u64,
    events: Vec<Event>,
}

impl Engine {
    /// Parse (+ soft sema) source into a runnable engine.
    pub fn from_source(src: &str) -> Result<Self> {
        let tu = parse(src)?;
        let sema = analyze(&tu);
        // Soft: still allow run if only unused warnings; but fail on undeclared critical?
        // Keep strict only for hard errors that would crash — for MVP ignore sema fail and let runtime catch.
        let _ = sema;

        let mut functions = HashMap::new();
        let mut classes = HashMap::new();

        for item in tu.items {
            match item {
                Item::Function(f) => {
                    functions.insert(f.name.name.clone(), f);
                }
                Item::Class(c) => {
                    let cname = c.name.name.clone();
                    for m in &c.members {
                        if let Member::Function(f) = m {
                            let q = format!("{cname}::{}", f.name.name);
                            functions.insert(q, f.clone());
                        }
                    }
                    classes.insert(cname, c);
                }
                Item::Decl(d) => {
                    // globals initialized lazily on first access / at start
                    let _ = d;
                }
                Item::UsingNamespace { .. } => {}
            }
        }

        let mut engine = Self {
            functions,
            classes,
            globals: HashMap::new(),
            heap: Heap::default(),
            map_value_tys: HashMap::new(),
            stack: Vec::new(),
            next_call_id: 0,
            events: Vec::new(),
        };

        // Init global decls
        let tu2 = parse(src)?;
        for item in &tu2.items {
            if let Item::Decl(d) = item {
                engine.exec_global_decl(d)?;
            }
        }

        Ok(engine)
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn run_main(&mut self) -> Result<Value> {
        self.call("main", &[])
    }

    /// Call `name` or `Class::method`. Methods get a fresh class instance as `this`.
    pub fn call(&mut self, name: &str, args: &[Value]) -> Result<Value> {
        if let Some((class, method)) = name.split_once("::") {
            let id = self.heap.alloc(Object::Class {
                name: class.to_string(),
                fields: HashMap::new(),
            });
            self.emit_alloc(id, class, Span::new(0, 0));
            let q = format!("{class}::{method}");
            self.call_fn(&q, args, Some(Value::Object(id)))
        } else {
            self.call_fn(name, args, None)
        }
    }

    /// Helper: build a heap `vector` from values.
    pub fn make_vector(&mut self, elems: Vec<Value>) -> Value {
        let id = self.heap.alloc(Object::Vector(elems));
        self.emit_alloc(id, "vector", Span::new(0, 0));
        Value::Object(id)
    }

    pub fn vector_as_ints(&self, v: &Value) -> Result<Vec<i64>> {
        let Value::Object(id) = v else {
            return Err(RuntimeError::new("expected vector object"));
        };
        match self.heap.get(*id) {
            Some(Object::Vector(elems)) => elems
                .iter()
                .map(|e| e.as_int().map_err(RuntimeError::new))
                .collect(),
            _ => Err(RuntimeError::new("expected vector")),
        }
    }

    fn emit(&mut self, e: Event) {
        self.events.push(e);
    }

    fn current_call_id(&self) -> Option<u64> {
        self.stack.last().map(|f| f.call_id)
    }

    fn emit_alloc(&mut self, id: ObjId, kind: impl Into<String>, span: Span) {
        let (size, elems, entries) = self
            .heap
            .get(id)
            .map(|o| o.alloc_snapshot())
            .unwrap_or((0, vec![], vec![]));
        self.emit(Event::Alloc {
            call_id: self.current_call_id(),
            id,
            kind: kind.into(),
            size,
            elems,
            entries,
            span,
        });
    }

    /// Same shape as `stl::Ctx::query` (index / method reads).
    fn query(
        &mut self,
        container: Value,
        op: impl Into<String>,
        key: Option<Value>,
        result: Value,
        span: Span,
    ) -> Value {
        let call_id = self.current_call_id();
        stl::Ctx {
            heap: &mut self.heap,
            events: &mut self.events,
            call_id,
        }
        .query(container, op, key, result, span)
    }

    fn exec_global_decl(&mut self, d: &Decl) -> Result<()> {
        for decl in &d.declarators {
            let val = if let Some(init) = &decl.init {
                self.eval_expr(init)?
            } else {
                self.default_value_for_type(&d.ty)?
            };
            self.globals.insert(decl.name.name.clone(), val.clone());
            self.emit(Event::VarCreate {
                call_id: self.current_call_id(),
                name: decl.name.name.clone(),
                value: val,
                span: decl.span,
            });
        }
        Ok(())
    }

    fn call_fn(&mut self, name: &str, args: &[Value], this: Option<Value>) -> Result<Value> {
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
    fn resolve_fn_call(&self, name: &str) -> Result<(String, Option<Value>)> {
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

    fn exec_block(&mut self, block: &Block) -> Result<Flow> {
        self.emit(Event::ScopeEnter {
            call_id: self.current_call_id(),
            span: block.span,
        });
        for stmt in &block.stmts {
            match self.exec_stmt(stmt)? {
                Flow::Next => {}
                other => {
                    self.emit(Event::ScopeExit {
                        call_id: self.current_call_id(),
                        span: block.span,
                    });
                    return Ok(other);
                }
            }
        }
        self.emit(Event::ScopeExit {
            call_id: self.current_call_id(),
            span: block.span,
        });
        Ok(Flow::Next)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow> {
        // Nested blocks emit ScopeEnter instead of a single Step.
        if !matches!(stmt, Stmt::Block(_)) {
            self.emit(Event::Step {
                call_id: self.current_call_id(),
                span: stmt.span(),
            });
        }
        match stmt {
            Stmt::Block(b) => self.exec_block(b),
            Stmt::Decl(d) => {
                for decl in &d.declarators {
                    let val = self.eval_decl_init(&d.ty, decl)?;
                    self.define_local(&decl.name.name, val, decl.span)?;
                }
                Ok(Flow::Next)
            }
            Stmt::Expr { expr, .. } => {
                let _ = self.eval_expr(expr)?;
                Ok(Flow::Next)
            }
            Stmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Void,
                };
                Ok(Flow::Return(v))
            }
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::Continue),
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let c = self.eval_expr(cond)?;
                let then_taken = c.as_bool().map_err(RuntimeError::new)?;
                self.emit(Event::Branch {
                    call_id: self.current_call_id(),
                    then_taken,
                    span: *span,
                });
                if then_taken {
                    self.exec_stmt(then_branch)
                } else if let Some(e) = else_branch {
                    self.exec_stmt(e)
                } else {
                    Ok(Flow::Next)
                }
            }
            Stmt::While {
                cond, body, span, ..
            } => {
                loop {
                    let c = self.eval_expr(cond)?;
                    if !c.as_bool().map_err(RuntimeError::new)? {
                        break;
                    }
                    self.emit(Event::LoopIter {
                        call_id: self.current_call_id(),
                        span: *span,
                    });
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Next)
            }
            Stmt::DoWhile {
                body, cond, span, ..
            } => {
                loop {
                    self.emit(Event::LoopIter {
                        call_id: self.current_call_id(),
                        span: *span,
                    });
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                    let c = self.eval_expr(cond)?;
                    if !c.as_bool().map_err(RuntimeError::new)? {
                        break;
                    }
                }
                Ok(Flow::Next)
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                span,
                ..
            } => {
                match init {
                    Some(ForInit::Decl(d)) => {
                        let _ = self.exec_stmt(&Stmt::Decl(d.clone()))?;
                    }
                    Some(ForInit::Expr(e)) => {
                        let _ = self.eval_expr(e)?;
                    }
                    None => {}
                }
                loop {
                    if let Some(c) = cond {
                        let cv = self.eval_expr(c)?;
                        if !cv.as_bool().map_err(RuntimeError::new)? {
                            break;
                        }
                    }
                    self.emit(Event::LoopIter {
                        call_id: self.current_call_id(),
                        span: *span,
                    });
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                    if let Some(s) = step {
                        let _ = self.eval_expr(s)?;
                    }
                }
                Ok(Flow::Next)
            }
        }
    }

    fn define_local(&mut self, name: &str, val: Value, span: Span) -> Result<()> {
        if let Value::Ref(addr) = &val {
            if let Some(slot) = Self::address_to_slot(addr) {
                self.emit(Event::RefBind {
                    call_id: self.current_call_id(),
                    name: name.to_string(),
                    target: slot,
                    span,
                });
            }
        }
        if let Value::Ptr(addr) = &val {
            self.emit(Event::PtrMove {
                call_id: self.current_call_id(),
                name: name.to_string(),
                to: Value::Ptr(addr.clone()),
                span,
            });
        }
        if let Some(frame) = self.stack.last_mut() {
            frame.locals.insert(name.to_string(), val.clone());
        } else {
            self.globals.insert(name.to_string(), val.clone());
        }
        self.emit(Event::VarCreate {
            call_id: self.current_call_id(),
            name: name.to_string(),
            value: val,
            span,
        });
        Ok(())
    }

    fn lookup_raw(&self, name: &str) -> Result<Value> {
        for frame in self.stack.iter().rev() {
            if let Some(v) = frame.locals.get(name) {
                return Ok(v.clone());
            }
        }
        if let Some(v) = self.globals.get(name) {
            return Ok(v.clone());
        }
        Err(RuntimeError::new(format!("undefined variable `{name}`")))
    }

    #[allow(dead_code)]
    fn lookup(&self, name: &str) -> Result<Value> {
        let v = self.lookup_raw(name)?;
        match v {
            Value::Ref(addr) => self.load_address(&addr),
            other => Ok(other),
        }
    }

    fn assign_name(&mut self, name: &str, val: Value, span: Span) -> Result<()> {
        // Write-through references.
        if let Ok(Value::Ref(addr)) = self.lookup_raw(name) {
            return self.store_address(&addr, val, span);
        }

        for i in (0..self.stack.len()).rev() {
            if self.stack[i].locals.contains_key(name) {
                let old = self.stack[i].locals.get(name).cloned();
                if matches!(val, Value::Ptr(_)) {
                    self.emit(Event::PtrMove {
                        call_id: self.current_call_id(),
                        name: name.to_string(),
                        to: val.clone(),
                        span,
                    });
                }
                self.stack[i].locals.insert(name.to_string(), val.clone());
                self.emit(Event::VarAssign {
                    call_id: self.current_call_id(),
                    name: name.to_string(),
                    old: old.clone(),
                    value: val.clone(),
                    span,
                });
                self.emit(Event::Write {
                    call_id: self.current_call_id(),
                    slot: Slot::Local {
                        name: name.to_string(),
                    },
                    old,
                    value: val,
                    span,
                });
                return Ok(());
            }
        }
        if self.globals.contains_key(name) {
            let old = self.globals.get(name).cloned();
            if matches!(val, Value::Ptr(_)) {
                self.emit(Event::PtrMove {
                    call_id: self.current_call_id(),
                    name: name.to_string(),
                    to: val.clone(),
                    span,
                });
            }
            self.globals.insert(name.to_string(), val.clone());
            self.emit(Event::VarAssign {
                call_id: self.current_call_id(),
                name: name.to_string(),
                old: old.clone(),
                value: val.clone(),
                span,
            });
            self.emit(Event::Write {
                call_id: self.current_call_id(),
                slot: Slot::Global {
                    name: name.to_string(),
                },
                old,
                value: val,
                span,
            });
            return Ok(());
        }
        Err(RuntimeError::at(
            span,
            format!("assignment to undeclared `{name}`"),
        ))
    }

    fn write_lvalue(&mut self, lv: &LValue, val: Value, span: Span) -> Result<()> {
        match lv {
            LValue::Name(n) => self.assign_name(n, val, span),
            LValue::Index { obj, index } => {
                let Some(Object::Vector(elems)) = self.heap.get_mut(*obj) else {
                    return Err(RuntimeError::at(span, "index assignment on non-vector"));
                };
                if *index >= elems.len() {
                    return Err(RuntimeError::at(span, "index out of bounds"));
                }
                let old = elems[*index].clone();
                elems[*index] = val.clone();
                self.emit(Event::Write {
                    call_id: self.current_call_id(),
                    slot: Slot::Index {
                        obj: *obj,
                        index: *index,
                    },
                    old: Some(old.clone()),
                    value: val.clone(),
                    span,
                });
                self.emit(Event::ContainerMod {
                    call_id: self.current_call_id(),
                    container: Value::Object(*obj),
                    kind: "index_assign".into(),
                    index: Some(*index),
                    key: Some(Value::Int(*index as i64)),
                    old: Some(old),
                    value: Some(val),
                    span,
                });
                Ok(())
            }
            LValue::MapEntry { obj, key } => {
                let key_s = key.to_string();
                let old = match self.heap.get(*obj) {
                    Some(Object::Map(m)) => m.get(key).cloned(),
                    Some(Object::UnorderedMap(m)) => m.get(key).cloned(),
                    _ => None,
                };
                match self.heap.get_mut(*obj) {
                    Some(Object::Map(m)) => {
                        m.insert(key.clone(), val.clone());
                    }
                    Some(Object::UnorderedMap(m)) => {
                        m.insert(key.clone(), val.clone());
                    }
                    _ => return Err(RuntimeError::at(span, "map entry assign on non-map")),
                }
                self.emit(Event::Write {
                    call_id: self.current_call_id(),
                    slot: Slot::MapEntry {
                        obj: *obj,
                        key: key_s,
                    },
                    old: old.clone(),
                    value: val.clone(),
                    span,
                });
                self.emit(Event::ContainerMod {
                    call_id: self.current_call_id(),
                    container: Value::Object(*obj),
                    kind: "map_assign".into(),
                    index: None,
                    key: Some(key.to_value()),
                    old,
                    value: Some(val),
                    span,
                });
                Ok(())
            }
            LValue::Field { obj, field } => {
                let old = match self.heap.get(*obj) {
                    Some(Object::Class { fields, .. }) => fields.get(field).cloned(),
                    Some(Object::Pair { first, second }) => match field.as_str() {
                        "first" => Some(first.clone()),
                        "second" => Some(second.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                match self.heap.get_mut(*obj) {
                    Some(Object::Class { fields, .. }) => {
                        fields.insert(field.clone(), val.clone());
                    }
                    Some(Object::Pair { first, second }) => match field.as_str() {
                        "first" => *first = val.clone(),
                        "second" => *second = val.clone(),
                        _ => {
                            return Err(RuntimeError::at(
                                span,
                                format!("no field `{field}` on pair"),
                            ));
                        }
                    },
                    _ => return Err(RuntimeError::at(span, "field assign on bad object")),
                }
                self.emit(Event::Write {
                    call_id: self.current_call_id(),
                    slot: Slot::Field {
                        obj: *obj,
                        field: field.clone(),
                    },
                    old: old.clone(),
                    value: val.clone(),
                    span,
                });
                self.emit(Event::VarAssign {
                    call_id: self.current_call_id(),
                    name: field.clone(),
                    old,
                    value: val,
                    span,
                });
                Ok(())
            }
        }
    }

    fn slot_of(lv: &LValue) -> Slot {
        match lv {
            LValue::Name(n) => Slot::Local { name: n.clone() },
            LValue::Index { obj, index } => Slot::Index {
                obj: *obj,
                index: *index,
            },
            LValue::MapEntry { obj, key } => Slot::MapEntry {
                obj: *obj,
                key: key.to_string(),
            },
            LValue::Field { obj, field } => Slot::Field {
                obj: *obj,
                field: field.clone(),
            },
        }
    }

    fn address_to_slot(addr: &Address) -> Option<Slot> {
        Some(match addr {
            Address::Null => return None,
            Address::Stack { name, .. } => Slot::Local {
                name: name.clone(),
            },
            Address::Heap(id) => Slot::Object { obj: *id },
            Address::Index { obj, index } => Slot::Index {
                obj: *obj,
                index: *index,
            },
            Address::Field { obj, field } => Slot::Field {
                obj: *obj,
                field: field.clone(),
            },
            Address::MapEntry { obj, key } => Slot::MapEntry {
                obj: *obj,
                key: key.clone(),
            },
        })
    }

    fn lvalue_to_address(&self, lv: &LValue) -> Result<Address> {
        Ok(match lv {
            LValue::Name(n) => {
                // Find which frame owns n
                for (i, frame) in self.stack.iter().enumerate().rev() {
                    if frame.locals.contains_key(n) {
                        // If local is already a Ref, address is the target
                        if let Some(Value::Ref(a)) = frame.locals.get(n) {
                            return Ok(a.clone());
                        }
                        return Ok(Address::Stack {
                            frame: i,
                            name: n.clone(),
                        });
                    }
                }
                if self.globals.contains_key(n) {
                    // treat as frame 0 global name via Stack depth usize::MAX? use name-only
                    return Ok(Address::Stack {
                        frame: usize::MAX,
                        name: n.clone(),
                    });
                }
                return Err(RuntimeError::new(format!(
                    "cannot take address of `{n}`"
                )));
            }
            LValue::Index { obj, index } => Address::Index {
                obj: *obj,
                index: *index,
            },
            LValue::MapEntry { obj, key } => Address::MapEntry {
                obj: *obj,
                key: key.to_string(),
            },
            LValue::Field { obj, field } => Address::Field {
                obj: *obj,
                field: field.clone(),
            },
        })
    }

    fn address_to_lvalue(&self, addr: &Address) -> Option<LValue> {
        match addr {
            Address::Stack { name, frame } if *frame == usize::MAX || self.stack.get(*frame).is_some() => {
                Some(LValue::Name(name.clone()))
            }
            Address::Index { obj, index } => Some(LValue::Index {
                obj: *obj,
                index: *index,
            }),
            Address::MapEntry { obj, key } => {
                // rebuild MapKey from string — int keys only for now
                let key = if let Ok(i) = key.parse::<i64>() {
                    MapKey::Int(i)
                } else {
                    MapKey::Str(key.clone())
                };
                Some(LValue::MapEntry { obj: *obj, key })
            }
            Address::Field { obj, field } => Some(LValue::Field {
                obj: *obj,
                field: field.clone(),
            }),
            Address::Heap(_) | Address::Null | Address::Stack { .. } => None,
        }
    }

    fn load_address(&self, addr: &Address) -> Result<Value> {
        match addr {
            Address::Null => Err(RuntimeError::new("null pointer dereference")),
            Address::Stack { frame, name } if *frame == usize::MAX => self
                .globals
                .get(name)
                .cloned()
                .ok_or_else(|| RuntimeError::new(format!("dangling ref `{name}`"))),
            Address::Stack { frame, name } => {
                let frame = self
                    .stack
                    .get(*frame)
                    .ok_or_else(|| RuntimeError::new("dangling stack address"))?;
                frame
                    .locals
                    .get(name)
                    .cloned()
                    .ok_or_else(|| RuntimeError::new(format!("dangling ref `{name}`")))
            }
            Address::Heap(id) => Ok(Value::Object(*id)),
            Address::Index { obj, index } => match self.heap.get(*obj) {
                Some(Object::Vector(e)) => e
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| RuntimeError::new("index out of bounds")),
                _ => Err(RuntimeError::new("bad index address")),
            },
            Address::Field { obj, field } => match self.heap.get(*obj) {
                Some(Object::Class { fields, .. }) => Ok(fields
                    .get(field)
                    .cloned()
                    .unwrap_or(Value::Int(0))),
                Some(Object::Pair { first, second }) => match field.as_str() {
                    "first" => Ok(first.clone()),
                    "second" => Ok(second.clone()),
                    _ => Err(RuntimeError::new("bad field")),
                },
                _ => Err(RuntimeError::new("bad field address")),
            },
            Address::MapEntry { obj, key } => {
                let mk = if let Ok(i) = key.parse::<i64>() {
                    MapKey::Int(i)
                } else if key == "true" || key == "false" {
                    MapKey::Bool(key == "true")
                } else if key.chars().count() == 1 {
                    MapKey::Char(key.chars().next().unwrap())
                } else {
                    MapKey::Str(key.clone())
                };
                match self.heap.get(*obj) {
                    Some(Object::Map(m)) => Ok(m.get(&mk).cloned().unwrap_or(Value::Int(0))),
                    Some(Object::UnorderedMap(m)) => {
                        Ok(m.get(&mk).cloned().unwrap_or(Value::Int(0)))
                    }
                    _ => Err(RuntimeError::new("bad map address")),
                }
            }
        }
    }

    fn store_address(&mut self, addr: &Address, val: Value, span: Span) -> Result<()> {
        match addr {
            Address::Null => Err(RuntimeError::at(span, "null pointer write")),
            Address::Stack { frame, name } if *frame == usize::MAX => {
                self.globals.insert(name.clone(), val.clone());
                self.emit(Event::Write {
                    call_id: self.current_call_id(),
                    slot: Slot::Global { name: name.clone() },
                    old: None,
                    value: val,
                    span,
                });
                Ok(())
            }
            Address::Stack { frame, name } => {
                let frame = self
                    .stack
                    .get_mut(*frame)
                    .ok_or_else(|| RuntimeError::at(span, "dangling stack address"))?;
                let old = frame.locals.insert(name.clone(), val.clone());
                self.emit(Event::Write {
                    call_id: self.current_call_id(),
                    slot: Slot::Local { name: name.clone() },
                    old,
                    value: val,
                    span,
                });
                Ok(())
            }
            Address::Heap(_) => Err(RuntimeError::at(span, "cannot store through object address")),
            other => {
                let Some(lv) = self.address_to_lvalue(other) else {
                    return Err(RuntimeError::at(span, "cannot store to address"));
                };
                self.write_lvalue(&lv, val, span)
            }
        }
    }

    fn type_is_ref(ty: &Type) -> bool {
        match ty {
            Type::Reference { .. } => true,
            Type::Const { inner, .. } => Self::type_is_ref(inner),
            _ => false,
        }
    }

    fn type_is_ptr(ty: &Type) -> bool {
        match ty {
            Type::Pointer { .. } => true,
            Type::Const { inner, .. } => Self::type_is_ptr(inner),
            Type::Reference { inner, .. } => Self::type_is_ptr(inner),
            _ => false,
        }
    }

    fn bind_param_value(
        &mut self,
        ty: &Type,
        arg: &Value,
        name: &str,
        span: Span,
    ) -> Result<Value> {
        if Self::type_is_ref(ty) {
            let addr = match arg {
                Value::Object(id) => Address::Heap(*id),
                Value::Ref(a) | Value::Ptr(a) => a.clone(),
                Value::Nullptr => Address::Null,
                other => {
                    return Err(RuntimeError::at(
                        span,
                        format!("cannot bind reference parameter to `{other}`"),
                    ))
                }
            };
            if let Some(slot) = Self::address_to_slot(&addr) {
                self.emit(Event::RefBind {
                    call_id: self.current_call_id(),
                    name: name.to_string(),
                    target: slot,
                    span,
                });
            }
            return Ok(Value::Ref(addr));
        }
        Ok(arg.clone())
    }

    fn eval_decl_init(&mut self, ty: &Type, decl: &InitDeclarator) -> Result<Value> {
        if Self::type_is_ref(ty) {
            let init = decl
                .init
                .as_ref()
                .ok_or_else(|| RuntimeError::at(decl.span, "reference must be initialized"))?;
            let (_v, lv) = self.eval_expr_lv(init)?;
            let lv = lv.ok_or_else(|| RuntimeError::at(decl.span, "cannot bind ref to rvalue"))?;
            let addr = self.lvalue_to_address(&lv)?;
            return Ok(Value::Ref(addr));
        }
        if let Some(init) = &decl.init {
            let v = self.eval_expr(init)?;
            if Self::type_is_ptr(ty) {
                return Ok(match v {
                    Value::Ptr(a) => Value::Ptr(a),
                    Value::Nullptr => Value::Ptr(Address::Null),
                    Value::Object(id) => Value::Ptr(Address::Heap(id)),
                    Value::Ref(a) => Value::Ptr(a),
                    other => {
                        return Err(RuntimeError::at(
                            decl.span,
                            format!("cannot initialize pointer from `{other}`"),
                        ))
                    }
                });
            }
            return Ok(v);
        }
        if Self::type_is_ptr(ty) {
            return Ok(Value::Ptr(Address::Null));
        }
        self.default_value_for_type(ty)
    }

    fn dealloc_owned_locals(&mut self, locals: &HashMap<String, Value>, keep: Option<&Value>) {
        let keep_id = match keep {
            Some(Value::Object(id)) => Some(*id),
            Some(Value::Ref(Address::Heap(id)) | Value::Ptr(Address::Heap(id))) => Some(*id),
            _ => None,
        };
        for (_name, v) in locals {
            if let Value::Object(id) = v {
                if Some(*id) == keep_id {
                    continue;
                }
                // Only free if no other references remain — check stack/globals roughly
                if self.value_mentions_obj(*id) {
                    continue;
                }
                if self.heap.free(*id).is_some() {
                    self.emit(Event::Dealloc {
                        call_id: self.current_call_id(),
                        id: *id,
                        span: Span::new(0, 0),
                    });
                }
            }
        }
    }

    fn value_mentions_obj(&self, id: ObjId) -> bool {
        let check = |v: &Value| matches!(v, Value::Object(x) if *x == id)
            || matches!(v, Value::Ptr(Address::Heap(x) | Address::Index { obj: x, .. } | Address::Field { obj: x, .. } | Address::MapEntry { obj: x, .. }) if *x == id)
            || matches!(v, Value::Ref(Address::Heap(x) | Address::Index { obj: x, .. } | Address::Field { obj: x, .. } | Address::MapEntry { obj: x, .. }) if *x == id);
        for f in &self.stack {
            if f.locals.values().any(check) {
                return true;
            }
        }
        self.globals.values().any(check)
    }

    fn builtin_swap(&mut self, a: &Expr, b: &Expr, span: Span) -> Result<Value> {
        let (va, la) = self.eval_expr_lv(a)?;
        let (vb, lb) = self.eval_expr_lv(b)?;
        let Some(la) = la else {
            return Err(RuntimeError::at(span, "swap arg is not an lvalue"));
        };
        let Some(lb) = lb else {
            return Err(RuntimeError::at(span, "swap arg is not an lvalue"));
        };
        self.write_lvalue(&la, vb.clone(), span)?;
        self.write_lvalue(&lb, va.clone(), span)?;
        self.emit(Event::Swap {
            call_id: self.current_call_id(),
            a: Self::slot_of(&la),
            b: Self::slot_of(&lb),
            value_a: va,
            value_b: vb,
            span,
        });
        Ok(Value::Void)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        let (v, _) = self.eval_expr_lv(expr)?;
        Ok(v)
    }

    fn eval_expr_lv(&mut self, expr: &Expr) -> Result<(Value, Option<LValue>)> {
        self.eval_expr_lv_opts(expr, true)
    }

    fn eval_expr_lv_opts(
        &mut self,
        expr: &Expr,
        emit_index_lookup: bool,
    ) -> Result<(Value, Option<LValue>)> {
        match expr {
            Expr::IntLit { value, .. } => Ok((Value::Int(*value as i64), None)),
            Expr::FloatLit { value, .. } => Ok((Value::Float(*value), None)),
            Expr::CharLit { value, .. } => Ok((Value::Char(*value), None)),
            Expr::StringLit { value, span } => {
                let id = self.heap.alloc(Object::String(value.clone()));
                self.emit_alloc(id, "string", *span);
                Ok((Value::Object(id), None))
            }
            Expr::BoolLit { value, .. } => Ok((Value::Bool(*value), None)),
            Expr::Nullptr { .. } => Ok((Value::Nullptr, None)),
            Expr::Name(path) => {
                if path.segments.len() == 1 {
                    let n = &path.segments[0].name;
                    let raw = self.lookup_raw(n)?;
                    match raw {
                        Value::Ref(addr) => {
                            let v = self.load_address(&addr)?;
                            Ok((v, self.address_to_lvalue(&addr)))
                        }
                        other => Ok((other, Some(LValue::Name(n.clone())))),
                    }
                } else {
                    Err(RuntimeError::at(
                        path.span,
                        "qualified names in expressions not supported yet",
                    ))
                }
            }
            Expr::Unary { op, expr, span } => {
                let (v, lv) = self.eval_expr_lv(expr)?;
                let out = match op {
                    UnaryOp::Plus => Value::Int(v.as_int().map_err(RuntimeError::new)?),
                    UnaryOp::Minus => Value::Int(-v.as_int().map_err(RuntimeError::new)?),
                    UnaryOp::Not => Value::Bool(!v.as_bool().map_err(RuntimeError::new)?),
                    UnaryOp::BitNot => Value::Int(!v.as_int().map_err(RuntimeError::new)?),
                    UnaryOp::PreInc => {
                        let n = v.as_int().map_err(RuntimeError::new)? + 1;
                        let nv = Value::Int(n);
                        if let Some(lv) = &lv {
                            self.write_lvalue(lv, nv.clone(), *span)?;
                        }
                        nv
                    }
                    UnaryOp::PreDec => {
                        let n = v.as_int().map_err(RuntimeError::new)? - 1;
                        let nv = Value::Int(n);
                        if let Some(lv) = &lv {
                            self.write_lvalue(lv, nv.clone(), *span)?;
                        }
                        nv
                    }
                    UnaryOp::PostInc => {
                        let old = v.clone();
                        let n = v.as_int().map_err(RuntimeError::new)? + 1;
                        if let Some(lv) = &lv {
                            self.write_lvalue(lv, Value::Int(n), *span)?;
                        }
                        old
                    }
                    UnaryOp::PostDec => {
                        let old = v.clone();
                        let n = v.as_int().map_err(RuntimeError::new)? - 1;
                        if let Some(lv) = &lv {
                            self.write_lvalue(lv, Value::Int(n), *span)?;
                        }
                        old
                    }
                    UnaryOp::AddrOf => {
                        let Some(lv) = lv else {
                            return Err(RuntimeError::at(*span, "cannot take address of rvalue"));
                        };
                        let addr = self.lvalue_to_address(&lv)?;
                        Value::Ptr(addr)
                    }
                    UnaryOp::Deref => match &v {
                        Value::Ptr(addr) | Value::Ref(addr) => {
                            let loaded = self.load_address(addr)?;
                            let out_lv = self.address_to_lvalue(addr);
                            return Ok((loaded, out_lv));
                        }
                        Value::Nullptr => {
                            return Err(RuntimeError::at(*span, "null pointer dereference"));
                        }
                        other => {
                            return Err(RuntimeError::at(
                                *span,
                                format!("cannot dereference `{other}`"),
                            ));
                        }
                    },
                };
                Ok((out, None))
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                let out = self.eval_binary(*op, &l, &r, *span)?;
                Ok((out, None))
            }
            Expr::Assign {
                op,
                left,
                right,
                span,
            } => {
                let (rv, _) = self.eval_expr_lv(right)?;
                // Pure `=` : resolve LHS without ContainerLookup (Write covers the store).
                // Compound assigns still lookup so the UI sees the old value read.
                let lookup = *op != AssignOp::Assign;
                let (lv_val, lv) = self.eval_expr_lv_opts(left, lookup)?;
                let Some(lv) = lv else {
                    return Err(RuntimeError::at(*span, "invalid assignment target"));
                };
                let new_val = if *op == AssignOp::Assign {
                    rv
                } else {
                    let bin = match op {
                        AssignOp::AddAssign => BinaryOp::Add,
                        AssignOp::SubAssign => BinaryOp::Sub,
                        AssignOp::MulAssign => BinaryOp::Mul,
                        AssignOp::DivAssign => BinaryOp::Div,
                        AssignOp::RemAssign => BinaryOp::Rem,
                        AssignOp::AndAssign => BinaryOp::BitAnd,
                        AssignOp::OrAssign => BinaryOp::BitOr,
                        AssignOp::XorAssign => BinaryOp::BitXor,
                        AssignOp::ShlAssign => BinaryOp::Shl,
                        AssignOp::ShrAssign => BinaryOp::Shr,
                        AssignOp::Assign => unreachable!(),
                    };
                    self.eval_binary(bin, &lv_val, &rv, *span)?
                };
                self.write_lvalue(&lv, new_val.clone(), *span)?;
                Ok((new_val, Some(lv)))
            }
            Expr::Call { callee, args, span } => {
                let arg_vals: Result<Vec<_>> = args.iter().map(|a| self.eval_expr(a)).collect();
                let arg_vals = arg_vals?;

                // member call: a.b(...) already parsed as Call { Member }
                if let Expr::Member {
                    base,
                    field,
                    arrow,
                    span: mspan,
                } = callee.as_ref()
                {
                    let base_v = self.eval_expr(base)?;
                    if *arrow {
                        // no real pointers yet
                        return Err(RuntimeError::at(*mspan, "`->` not supported yet"));
                    }
                    let ret = self.call_member(base_v, &field.name, &arg_vals, *span)?;
                    return Ok((ret, None));
                }

                if let Expr::Name(path) = callee.as_ref() {
                    let name = path
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join("::");
                    if (name == "swap" || name == "std::swap") && args.len() == 2 {
                        let ret = self.builtin_swap(&args[0], &args[1], *span)?;
                        return Ok((ret, None));
                    }
                    // Type-construction: vector / pair as function name
                    if name == "pair" && arg_vals.len() == 2 {
                        let id = self.heap.alloc(Object::Pair {
                            first: arg_vals[0].clone(),
                            second: arg_vals[1].clone(),
                        });
                        self.emit_alloc(id, "pair", *span);
                        return Ok((Value::Object(id), None));
                    }
                    let (resolved, this) = self.resolve_fn_call(&name)?;
                    let ret = self.call_fn(&resolved, &arg_vals, this)?;
                    return Ok((ret, None));
                }

                Err(RuntimeError::at(*span, "unsupported call"))
            }
            Expr::Index {
                base,
                index,
                span,
            } => {
                let b = self.eval_expr(base)?;
                let idx_val = self.eval_expr(index)?;
                let Value::Object(id) = b else {
                    return Err(RuntimeError::at(*span, "cannot index non-object"));
                };
                match self.heap.get(id).cloned() {
                    Some(Object::Vector(elems)) => {
                        let i = idx_val.as_int().map_err(RuntimeError::new)? as usize;
                        let v = elems
                            .get(i)
                            .cloned()
                            .ok_or_else(|| RuntimeError::at(*span, "index out of bounds"))?;
                        let v = if emit_index_lookup {
                            self.query(
                                Value::Object(id),
                                "index",
                                Some(idx_val),
                                v,
                                *span,
                            )
                        } else {
                            v
                        };
                        Ok((v, Some(LValue::Index { obj: id, index: i })))
                    }
                    Some(Object::String(s)) => {
                        let i = idx_val.as_int().map_err(RuntimeError::new)? as usize;
                        let ch = s
                            .chars()
                            .nth(i)
                            .ok_or_else(|| RuntimeError::at(*span, "index out of bounds"))?;
                        let v = if emit_index_lookup {
                            self.query(
                                Value::Object(id),
                                "index",
                                Some(idx_val),
                                Value::Char(ch),
                                *span,
                            )
                        } else {
                            Value::Char(ch)
                        };
                        Ok((v, None))
                    }
                    Some(Object::Map(_)) | Some(Object::UnorderedMap(_)) => {
                        let key = self.value_to_key(&idx_val)?;
                        let v = self.map_index_get_or_insert(id, &key, *span)?;
                        let v = if emit_index_lookup {
                            self.query(
                                Value::Object(id),
                                "index",
                                Some(idx_val),
                                v,
                                *span,
                            )
                        } else {
                            v
                        };
                        Ok((v, Some(LValue::MapEntry { obj: id, key })))
                    }
                    _ => Err(RuntimeError::at(*span, "type not subscriptable")),
                }
            }
            Expr::Member {
                base,
                field,
                arrow,
                span,
            } => {
                if *arrow {
                    return Err(RuntimeError::at(*span, "`->` not supported yet"));
                }
                let b = self.eval_expr(base)?;
                let Value::Object(id) = b else {
                    return Err(RuntimeError::at(*span, "member access on non-object"));
                };
                match self.heap.get(id) {
                    Some(Object::Pair { first, second }) => match field.name.as_str() {
                        "first" => Ok((
                            first.clone(),
                            Some(LValue::Field {
                                obj: id,
                                field: "first".into(),
                            }),
                        )),
                        "second" => Ok((
                            second.clone(),
                            Some(LValue::Field {
                                obj: id,
                                field: "second".into(),
                            }),
                        )),
                        _ => Err(RuntimeError::at(*span, "pair has first/second only")),
                    },
                    Some(Object::Class { fields, .. }) => {
                        let v = fields.get(&field.name).cloned().unwrap_or(Value::Int(0));
                        Ok((
                            v,
                            Some(LValue::Field {
                                obj: id,
                                field: field.name.clone(),
                            }),
                        ))
                    }
                    // member methods without call are not values
                    _ => Err(RuntimeError::at(
                        *span,
                        format!("cannot read field `{}`", field.name),
                    )),
                }
            }
            Expr::Cast { expr, .. } => {
                let v = self.eval_expr(expr)?;
                Ok((v, None))
            }
            Expr::InitList { elems, span } => {
                let mut vs = Vec::new();
                for e in elems {
                    vs.push(self.eval_expr(e)?);
                }
                let id = self.heap.alloc(Object::Vector(vs));
                self.emit_alloc(id, "vector", *span);
                Ok((Value::Object(id), None))
            }
        }
    }

    fn value_to_key(&self, v: &Value) -> Result<MapKey> {
        MapKey::from_value(v, |id| self.heap.string_value(id)).map_err(RuntimeError::new)
    }

    fn call_member(
        &mut self,
        base: Value,
        method: &str,
        args: &[Value],
        span: Span,
    ) -> Result<Value> {
        let Value::Object(id) = base.clone() else {
            return Err(RuntimeError::at(span, "method call on non-object"));
        };
        let kind = self
            .heap
            .get(id)
            .map(|o| o.kind_name())
            .unwrap_or("?");

        if kind == "class" {
            let name = match self.heap.get(id) {
                Some(Object::Class { name, .. }) => name.clone(),
                _ => return Err(RuntimeError::at(span, "dangling object")),
            };
            let q = format!("{name}::{method}");
            return self.call_fn(&q, args, Some(base));
        }

        if self.heap.get(id).is_none() {
            return Err(RuntimeError::at(span, "dangling object"));
        }

        let call_id = self.current_call_id();
        let mut ctx = stl::Ctx {
            heap: &mut self.heap,
            events: &mut self.events,
            call_id,
        };
        stl::call_method(&mut ctx, id, base, kind, method, args, span)
    }

    fn eval_binary(&mut self, op: BinaryOp, l: &Value, r: &Value, span: Span) -> Result<Value> {
        use BinaryOp::*;
        match op {
            Add | Sub | Mul | Div | Rem | BitAnd | BitXor | BitOr | Shl | Shr => {
                let a = l.as_int().map_err(RuntimeError::new)?;
                let b = r.as_int().map_err(RuntimeError::new)?;
                let n = match op {
                    Add => a + b,
                    Sub => a - b,
                    Mul => a * b,
                    Div => {
                        if b == 0 {
                            return Err(RuntimeError::at(span, "division by zero"));
                        }
                        a / b
                    }
                    Rem => {
                        if b == 0 {
                            return Err(RuntimeError::at(span, "division by zero"));
                        }
                        a % b
                    }
                    BitAnd => a & b,
                    BitXor => a ^ b,
                    BitOr => a | b,
                    Shl => a << b,
                    Shr => a >> b,
                    _ => unreachable!(),
                };
                Ok(Value::Int(n))
            }
            Lt | Gt | Le | Ge | Eq | Ne => {
                let result = match (l, r, op) {
                    (Value::Ptr(a), Value::Ptr(b), Eq) => a == b,
                    (Value::Ptr(a), Value::Ptr(b), Ne) => a != b,
                    (Value::Ptr(a), Value::Nullptr, Eq) | (Value::Nullptr, Value::Ptr(a), Eq) => {
                        *a == Address::Null
                    }
                    (Value::Ptr(a), Value::Nullptr, Ne) | (Value::Nullptr, Value::Ptr(a), Ne) => {
                        *a != Address::Null
                    }
                    (Value::Nullptr, Value::Nullptr, Eq) => true,
                    (Value::Nullptr, Value::Nullptr, Ne) => false,
                    (Value::Int(a), Value::Int(b), _) => cmp_ord(op, a.cmp(b)),
                    (Value::Float(a), Value::Float(b), _) => {
                        cmp_ord(op, a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    }
                    _ => {
                        let a = l.as_int().map_err(RuntimeError::new)?;
                        let b = r.as_int().map_err(RuntimeError::new)?;
                        cmp_ord(op, a.cmp(&b))
                    }
                };
                self.emit(Event::Compare {
                    call_id: self.current_call_id(),
                    op: format!("{op:?}"),
                    left: l.clone(),
                    right: r.clone(),
                    result,
                    span,
                });
                Ok(Value::Bool(result))
            }
            And => Ok(Value::Bool(
                l.as_bool().map_err(RuntimeError::new)?
                    && r.as_bool().map_err(RuntimeError::new)?,
            )),
            Or => Ok(Value::Bool(
                l.as_bool().map_err(RuntimeError::new)?
                    || r.as_bool().map_err(RuntimeError::new)?,
            )),
        }
    }

    fn default_value_for_type(&mut self, ty: &Type) -> Result<Value> {
        match ty {
            Type::Builtin { kind, .. } => Ok(match kind {
                BuiltinType::Bool => Value::Bool(false),
                BuiltinType::Float | BuiltinType::Double => Value::Float(0.0),
                BuiltinType::Void => Value::Void,
                BuiltinType::Char | BuiltinType::UnsignedChar => Value::Char('\0'),
                _ => Value::Int(0),
            }),
            Type::Named { path, args, .. } => {
                let name = path
                    .segments
                    .last()
                    .map(|s| s.name.as_str())
                    .unwrap_or("");
                if let Some(obj) = Object::empty_named(name) {
                    let kind = obj.kind_name().to_string();
                    let id = self.heap.alloc(obj);
                    // Remember V for map<K,V> / unordered_map<K,V> so operator[] can default-insert.
                    if (kind == "map" || kind == "unordered_map") && args.len() >= 2 {
                        self.map_value_tys.insert(id, args[1].clone());
                    }
                    self.emit_alloc(id, kind, path.span);
                    return Ok(Value::Object(id));
                }
                if self.classes.contains_key(name) {
                    let id = self.heap.alloc(Object::Class {
                        name: name.into(),
                        fields: std::collections::HashMap::new(),
                    });
                    return Ok(Value::Object(id));
                }
                Ok(Value::Int(0))
            }
            Type::Pointer { .. } => Ok(Value::Nullptr),
            Type::Reference { inner, .. } => self.default_value_for_type(inner),
            Type::Const { inner, .. } => self.default_value_for_type(inner),
        }
    }

    /// C++ `map::operator[]`: return existing value, or default-construct mapped type and insert.
    fn map_index_get_or_insert(
        &mut self,
        map_id: ObjId,
        key: &MapKey,
        span: Span,
    ) -> Result<Value> {
        let existing = match self.heap.get(map_id) {
            Some(Object::Map(m)) => m.get(key).cloned(),
            Some(Object::UnorderedMap(m)) => m.get(key).cloned(),
            _ => return Err(RuntimeError::at(span, "not a map")),
        };
        if let Some(v) = existing {
            return Ok(v);
        }

        let mapped_ty = self.map_value_tys.get(&map_id).cloned().unwrap_or(Type::Builtin {
            kind: BuiltinType::Int,
            span,
        });
        let def = self.default_value_for_type(&mapped_ty)?;
        match self.heap.get_mut(map_id) {
            Some(Object::Map(m)) => {
                m.insert(key.clone(), def.clone());
            }
            Some(Object::UnorderedMap(m)) => {
                m.insert(key.clone(), def.clone());
            }
            _ => return Err(RuntimeError::at(span, "not a map")),
        }
        self.emit(Event::Write {
            call_id: self.current_call_id(),
            slot: Slot::MapEntry {
                obj: map_id,
                key: key.to_string(),
            },
            old: None,
            value: def.clone(),
            span,
        });
        self.emit(Event::ContainerMod {
            call_id: self.current_call_id(),
            container: Value::Object(map_id),
            kind: "map_default_insert".into(),
            index: None,
            key: Some(key.to_value()),
            old: None,
            value: Some(def.clone()),
            span,
        });
        Ok(def)
    }
}

fn cmp_ord(op: BinaryOp, o: std::cmp::Ordering) -> bool {
    use BinaryOp::*;
    match op {
        Lt => o.is_lt(),
        Gt => o.is_gt(),
        Le => o.is_le(),
        Ge => o.is_ge(),
        Eq => o.is_eq(),
        Ne => o.is_ne(),
        _ => false,
    }
}
