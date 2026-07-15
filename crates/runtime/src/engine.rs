//! Tree-walking interpreter.

use crate::error::RuntimeError;
use crate::event::Event;
use crate::value::{Heap, Object, ObjId, Value};
use rscpp_ast::*;
use rscpp_parser::parse;
use rscpp_sema::analyze;
use std::collections::HashMap;

type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug)]
struct Frame {
    name: String,
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
    Field { obj: ObjId, field: String },
}

/// Parsed program + heap + call stack + event log.
pub struct Engine {
    functions: HashMap<String, FunctionDef>,
    classes: HashMap<String, ClassDef>,
    globals: HashMap<String, Value>,
    heap: Heap,
    stack: Vec<Frame>,
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
            stack: Vec::new(),
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
            self.emit(Event::Alloc {
                id,
                kind: class.to_string(),
                span: Span::new(0, 0),
            });
            let q = format!("{class}::{method}");
            self.call_fn(&q, args, Some(Value::Object(id)))
        } else {
            self.call_fn(name, args, None)
        }
    }

    /// Helper: build a heap `vector` from values.
    pub fn make_vector(&mut self, elems: Vec<Value>) -> Value {
        let id = self.heap.alloc(Object::Vector(elems));
        self.emit(Event::Alloc {
            id,
            kind: "vector".into(),
            span: Span::new(0, 0),
        });
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

    fn exec_global_decl(&mut self, d: &Decl) -> Result<()> {
        for decl in &d.declarators {
            let val = if let Some(init) = &decl.init {
                self.eval_expr(init)?
            } else {
                self.default_value_for_type(&d.ty)?
            };
            self.globals.insert(decl.name.name.clone(), val.clone());
            self.emit(Event::VarCreate {
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

        self.emit(Event::FnEnter {
            name: name.to_string(),
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
                locals.insert(n.name.clone(), a.clone());
                self.emit(Event::VarCreate {
                    name: n.name.clone(),
                    value: a.clone(),
                    span: n.span,
                });
            }
        }

        self.stack.push(Frame {
            name: name.to_string(),
            locals,
        });
        let flow = self.exec_block(&func.body)?;
        self.stack.pop();

        let ret = match flow {
            Flow::Return(v) => v,
            Flow::Next => Value::Int(0),
            Flow::Break | Flow::Continue => {
                return Err(RuntimeError::new("break/continue outside loop"));
            }
        };

        self.emit(Event::FnExit {
            name: name.to_string(),
            ret: ret.clone(),
            span: func.span,
        });
        Ok(ret)
    }

    fn exec_block(&mut self, block: &Block) -> Result<Flow> {
        // Block scope: snapshot? For simplicity share frame and remove names after.
        let introduced: Vec<String> = Vec::new();
        let _ = introduced;
        for stmt in &block.stmts {
            match self.exec_stmt(stmt)? {
                Flow::Next => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Next)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow> {
        match stmt {
            Stmt::Block(b) => self.exec_block(b),
            Stmt::Decl(d) => {
                for decl in &d.declarators {
                    let val = if let Some(init) = &decl.init {
                        self.eval_expr(init)?
                    } else {
                        self.default_value_for_type(&d.ty)?
                    };
                    self.define_local(&decl.name.name, val.clone(), decl.span);
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
                ..
            } => {
                let c = self.eval_expr(cond)?;
                if c.as_bool().map_err(RuntimeError::new)? {
                    self.exec_stmt(then_branch)
                } else if let Some(e) = else_branch {
                    self.exec_stmt(e)
                } else {
                    Ok(Flow::Next)
                }
            }
            Stmt::While { cond, body, .. } => {
                loop {
                    let c = self.eval_expr(cond)?;
                    if !c.as_bool().map_err(RuntimeError::new)? {
                        break;
                    }
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Next)
            }
            Stmt::DoWhile { body, cond, .. } => {
                loop {
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

    fn define_local(&mut self, name: &str, val: Value, span: Span) {
        if let Some(frame) = self.stack.last_mut() {
            frame.locals.insert(name.to_string(), val.clone());
        } else {
            self.globals.insert(name.to_string(), val.clone());
        }
        self.emit(Event::VarCreate {
            name: name.to_string(),
            value: val,
            span,
        });
    }

    fn lookup(&self, name: &str) -> Result<Value> {
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

    fn assign_name(&mut self, name: &str, val: Value, span: Span) -> Result<()> {
        for frame in self.stack.iter_mut().rev() {
            if frame.locals.contains_key(name) {
                frame.locals.insert(name.to_string(), val.clone());
                self.emit(Event::VarAssign {
                    name: name.to_string(),
                    value: val,
                    span,
                });
                return Ok(());
            }
        }
        if self.globals.contains_key(name) {
            self.globals.insert(name.to_string(), val.clone());
            self.emit(Event::VarAssign {
                name: name.to_string(),
                value: val,
                span,
            });
            return Ok(());
        }
        // Lexical assign creates local (C++ wouldn't) — treat as error
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
                elems[*index] = val.clone();
                self.emit(Event::ContainerMod {
                    container: Value::Object(*obj),
                    kind: "index_assign".into(),
                    detail: format!("[{index}] = {val}"),
                    span,
                });
                Ok(())
            }
            LValue::Field { obj, field } => {
                match self.heap.get_mut(*obj) {
                    Some(Object::Class { fields, .. }) => {
                        fields.insert(field.clone(), val.clone());
                    }
                    Some(Object::Pair { first, second }) => match field.as_str() {
                        "first" => *first = val.clone(),
                        "second" => *second = val.clone(),
                        _ => {
                            return Err(RuntimeError::at(span, format!("no field `{field}` on pair")));
                        }
                    },
                    _ => return Err(RuntimeError::at(span, "field assign on bad object")),
                }
                self.emit(Event::VarAssign {
                    name: field.clone(),
                    value: val,
                    span,
                });
                Ok(())
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        let (v, _) = self.eval_expr_lv(expr)?;
        Ok(v)
    }

    fn eval_expr_lv(&mut self, expr: &Expr) -> Result<(Value, Option<LValue>)> {
        match expr {
            Expr::IntLit { value, .. } => Ok((Value::Int(*value as i64), None)),
            Expr::FloatLit { value, .. } => Ok((Value::Float(*value), None)),
            Expr::CharLit { value, .. } => Ok((Value::Char(*value), None)),
            Expr::StringLit { value, span } => {
                let id = self.heap.alloc(Object::String(value.clone()));
                self.emit(Event::Alloc {
                    id,
                    kind: "string".into(),
                    span: *span,
                });
                Ok((Value::Object(id), None))
            }
            Expr::BoolLit { value, .. } => Ok((Value::Bool(*value), None)),
            Expr::Nullptr { .. } => Ok((Value::Nullptr, None)),
            Expr::Name(path) => {
                if path.segments.len() == 1 {
                    let n = &path.segments[0].name;
                    let v = self.lookup(n)?;
                    Ok((v, Some(LValue::Name(n.clone()))))
                } else {
                    // For now only simple names as values; Class::static later
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
                    UnaryOp::Deref | UnaryOp::AddrOf => {
                        return Err(RuntimeError::at(*span, "pointers not fully supported yet"));
                    }
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
                let (lv_val, lv) = self.eval_expr_lv(left)?;
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
                    // Type-construction: vector / pair as function name
                    if name == "pair" && arg_vals.len() == 2 {
                        let id = self.heap.alloc(Object::Pair {
                            first: arg_vals[0].clone(),
                            second: arg_vals[1].clone(),
                        });
                        self.emit(Event::Alloc {
                            id,
                            kind: "pair".into(),
                            span: *span,
                        });
                        return Ok((Value::Object(id), None));
                    }
                    let ret = self.call_fn(&name, &arg_vals, None)?;
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
                let i = self.eval_expr(index)?.as_int().map_err(RuntimeError::new)? as usize;
                let Value::Object(id) = b else {
                    return Err(RuntimeError::at(*span, "cannot index non-object"));
                };
                match self.heap.get(id) {
                    Some(Object::Vector(elems)) => {
                        let v = elems
                            .get(i)
                            .cloned()
                            .ok_or_else(|| RuntimeError::at(*span, "index out of bounds"))?;
                        Ok((v, Some(LValue::Index { obj: id, index: i })))
                    }
                    Some(Object::String(s)) => {
                        let ch = s
                            .chars()
                            .nth(i)
                            .ok_or_else(|| RuntimeError::at(*span, "index out of bounds"))?;
                        Ok((Value::Char(ch), None))
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
                self.emit(Event::Alloc {
                    id,
                    kind: "vector".into(),
                    span: *span,
                });
                Ok((Value::Object(id), None))
            }
        }
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
        match self.heap.get(id).cloned() {
            Some(Object::Vector(elems)) => match method {
                "size" => Ok(Value::Int(elems.len() as i64)),
                "empty" => Ok(Value::Bool(elems.is_empty())),
                "push_back" => {
                    let v = args
                        .first()
                        .cloned()
                        .ok_or_else(|| RuntimeError::at(span, "push_back needs an argument"))?;
                    if let Some(Object::Vector(e)) = self.heap.get_mut(id) {
                        e.push(v.clone());
                    }
                    self.emit(Event::ContainerMod {
                        container: base,
                        kind: "push_back".into(),
                        detail: format!("{v}"),
                        span,
                    });
                    Ok(Value::Void)
                }
                "pop_back" => {
                    if let Some(Object::Vector(e)) = self.heap.get_mut(id) {
                        e.pop();
                    }
                    self.emit(Event::ContainerMod {
                        container: base,
                        kind: "pop_back".into(),
                        detail: String::new(),
                        span,
                    });
                    Ok(Value::Void)
                }
                "clear" => {
                    if let Some(Object::Vector(e)) = self.heap.get_mut(id) {
                        e.clear();
                    }
                    Ok(Value::Void)
                }
                _ => Err(RuntimeError::at(
                    span,
                    format!("unknown vector method `{method}`"),
                )),
            },
            Some(Object::String(s)) => match method {
                "size" => Ok(Value::Int(s.len() as i64)),
                "empty" => Ok(Value::Bool(s.is_empty())),
                _ => Err(RuntimeError::at(
                    span,
                    format!("unknown string method `{method}`"),
                )),
            },
            Some(Object::Class { name, .. }) => {
                let q = format!("{name}::{method}");
                self.call_fn(&q, args, Some(base))
            }
            _ => Err(RuntimeError::at(span, "unsupported method receiver")),
        }
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
                let result = match (l, r) {
                    (Value::Int(a), Value::Int(b)) => cmp_ord(op, a.cmp(b)),
                    (Value::Float(a), Value::Float(b)) => {
                        cmp_ord(op, a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    }
                    _ => {
                        let a = l.as_int().map_err(RuntimeError::new)?;
                        let b = r.as_int().map_err(RuntimeError::new)?;
                        cmp_ord(op, a.cmp(&b))
                    }
                };
                self.emit(Event::Compare {
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
            Type::Named { path, .. } => {
                let name = path
                    .segments
                    .last()
                    .map(|s| s.name.as_str())
                    .unwrap_or("");
                match name {
                    "vector" => Ok(self.make_vector(vec![])),
                    "string" => {
                        let id = self.heap.alloc(Object::String(String::new()));
                        Ok(Value::Object(id))
                    }
                    "pair" => {
                        let id = self.heap.alloc(Object::Pair {
                            first: Value::Int(0),
                            second: Value::Int(0),
                        });
                        Ok(Value::Object(id))
                    }
                    other => {
                        if self.classes.contains_key(other) {
                            let id = self.heap.alloc(Object::Class {
                                name: other.into(),
                                fields: HashMap::new(),
                            });
                            Ok(Value::Object(id))
                        } else {
                            Ok(Value::Int(0))
                        }
                    }
                }
            }
            Type::Pointer { .. } => Ok(Value::Nullptr),
            Type::Reference { inner, .. } => self.default_value_for_type(inner),
            Type::Const { inner, .. } => self.default_value_for_type(inner),
        }
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
