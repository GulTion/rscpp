//! Tree-walking interpreter.

use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::builtins;
use crate::value::{Address, Heap, MapKey, Object, ObjId, Value};
use rscpp_ast::*;
use rscpp_parser::parse;
use rscpp_sema::analyze;
use std::collections::{HashMap, HashSet};

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
    /// Decremented on each statement / loop iter; 0 → error (browser safety).
    fuel: u64,
}

/// Default step budget for `run` / loops (browser-safe).
pub const DEFAULT_FUEL: u64 = 100_000;

impl Engine {
    /// Parse + analyze source into a runnable engine.
    pub fn from_source(src: &str) -> Result<Self> {
        Self::from_source_with_fuel(src, DEFAULT_FUEL)
    }

    pub fn from_source_with_fuel(src: &str, fuel: u64) -> Result<Self> {
        let tu = parse(src)?;
        let sema = analyze(&tu);
        if !sema.ok() {
            let e = &sema.errors[0];
            return Err(RuntimeError::at(e.span, e.message.clone()));
        }

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
            fuel,
        };

        let tu2 = parse(src)?;
        for item in &tu2.items {
            if let Item::Decl(d) = item {
                engine.exec_global_decl(d)?;
            }
        }

        Ok(engine)
    }

    pub fn set_fuel(&mut self, fuel: u64) {
        self.fuel = fuel;
    }

    fn burn(&mut self) -> Result<()> {
        if self.fuel == 0 {
            return Err(RuntimeError::new(
                "step limit exceeded (possible infinite loop)",
            ));
        }
        self.fuel -= 1;
        Ok(())
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

    /// Helper: build a heap `string`.
    pub fn make_string(&mut self, s: String) -> Value {
        let id = self.heap.alloc(Object::String(s));
        self.emit_alloc(id, "string", Span::new(0, 0));
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
}

mod global;
mod call;
mod exec;
mod vars;
mod addr;
mod bind;
mod algo;
mod expr;
mod types;
