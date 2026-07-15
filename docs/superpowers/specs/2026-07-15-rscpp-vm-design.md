# rscpp VM Design

**Date:** 2026-07-15  
**Crate:** `rscpp-vm`  
**Phase:** 8

## Problem

The tree-walker in `rscpp-runtime` is clear but harder to pause/resume and to ship as a compact WASM interpreter loop. A bytecode VM gives a single dispatch loop, clearer stepping, and a path to serialize programs later.

## Approach

```
source → parser → AST → bytecode Chunk(s) → stack VM → Value + Event stream
```

- Reuse `rscpp-runtime::{Value, Heap, Object, Event, Address}` — no second object model.
- One `Chunk` per function: `ops`, parallel `spans`, `constants`.
- Stack machine + call frames (`locals: Vec<Value>`).
- STL stays as **native methods** (`Op::CallMethod`) implemented against the same heap.

## Non-goals

- Full C++ ABI / unwind
- Optimizing compiler
- Replacing the tree-walker yet (both coexist; `Engine` remains for parity tests)

## API

```rust
pub fn run_main(src: &str) -> Result<(Value, Vec<Event>), VmError>;
pub fn call(src: &str, name: &str, args: &[Value]) -> Result<(Value, Vec<Event>), VmError>;
```

## Why better than JSCPP

JSCPP steps generators over AST nodes. We compile once to dense ops with spans, then run a tight loop — better for WASM and for evented scrubbing.
