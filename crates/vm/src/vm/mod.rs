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
    call_id: u64,
    parent_id: Option<u64>,
}

pub struct Vm {
    program: Program,
    heap: Heap,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    next_call_id: u64,
    events: Vec<Event>,
}

impl Vm {
    pub fn new(program: Program) -> Self {
        Self {
            program,
            heap: Heap::default(),
            stack: Vec::new(),
            frames: Vec::new(),
            next_call_id: 0,
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

        let parent_id = self.frames.last().map(|f| f.call_id);
        let call_id = self.next_call_id;
        self.next_call_id += 1;

        self.emit(Event::FnEnter {
            name: chunk.name.clone(),
            call_id,
            parent_id,
            args: args.to_vec(),
            span: Span::new(0, 0),
        });

        self.frames.push(Frame {
            func,
            ip: 0,
            locals,
            stack_base: self.stack.len(),
            call_id,
            parent_id,
        });

        self.run_loop()
    }

    fn emit(&mut self, e: Event) {
        self.events.push(e);
    }

    fn current_call_id(&self) -> Option<u64> {
        self.frames.last().map(|f| f.call_id)
    }

    fn emit_alloc(&mut self, id: rscpp_runtime::ObjId, kind: impl Into<String>, span: Span) {
        let (size, elems, entries) =
            self.heap
                .get(id)
                .map(|o| o.alloc_snapshot())
                .unwrap_or((0, vec![], vec![]));
        self.emit(Event::Alloc {
            id,
            kind: kind.into(),
            size,
            elems,
            entries,
            span,
        });
    }
}

mod ops;
mod run;

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
