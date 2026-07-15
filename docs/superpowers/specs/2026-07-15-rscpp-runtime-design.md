# rscpp Runtime Design

**Date:** 2026-07-15  
**Crate:** `rscpp-runtime`  
**Phase:** 5 (interpreter; VM later)

## Problem

JSCPP executes via generator visitors over a dynamically typed `CRuntime`. That works in JS but couples stepping, types, and library code. We need a Rust interpreter that evaluates the AST and emits a UI-agnostic event stream for the future visualizer.

## Goals

- Tree-walk `TranslationUnit` / call a named function (e.g. `main`, `Solution::twoSum`).
- Values: primitives, references, heap objects (`vector`, `pair`, `string`, class instances).
- Emit events: var create/assign, fn enter/exit, comparisons, container mods, alloc/dealloc.
- **v1 (visualizer):** `Step`, `ScopeEnter`/`ScopeExit`, `Branch`, `LoopIter`, `Write` (slot + old/new), `Swap`, richer `ContainerMod`, stubs for `RefBind`/`PtrMove`.
- Interpreter never imports UI code — only pushes to an event log.

## Non-goals (this phase)

- Bytecode VM (phase 8)
- Full STL (phase 6 — stub `vector`/`pair`/`string` only)
- Fancy memory model / GC visualization beyond alloc ids (phase 7)
- Preprocessor / `#include` (stdio stubs only as needed)

## API

```rust
pub struct Engine { /* ... */ }

impl Engine {
    pub fn from_source(src: &str) -> Result<Self, RuntimeError>;
    pub fn run_main(&mut self) -> Result<Value, RuntimeError>;
    pub fn call(&mut self, name: &str, args: &[Value]) -> Result<Value, RuntimeError>;
    pub fn events(&self) -> &[Event];
    pub fn take_events(&mut self) -> Vec<Event>;
}
```

## Why better than JSCPP

Static AST + explicit `Value`/`Heap`; events are a first-class stream; WASM-friendly (no generators); sema can run before execute.
