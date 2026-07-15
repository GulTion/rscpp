# rscpp WASM Design

**Date:** 2026-07-15  
**Crate:** `rscpp-wasm`  
**Phase:** 9

## Problem

Phases 1–8 run only in native Rust (CLI / tests). The visualizer needs the same pipeline in the browser: pass C++ source, get a return value and an event stream, with no UI code inside the engine.

## Goals

- Ship a `wasm-bindgen` crate the browser can call.
- **Run-only API:** one function that executes `main` (tree-walker) and returns JSON-friendly results.
- Serialize `Event` / `Value` (and nested slots) so JS can consume them without Rust types.
- Keep the engine I/O-free for v1: no stdin, no stdout, no `cin`/`cout`.

## Non-goals (this phase)

- Stepper / pause-resume
- Separate `lex` / `parse` / `sema` / `vm` exports
- Bytecode VM path from WASM
- Preprocessor / `#include`
- Console I/O (stdout capture can be a later follow-up)
- Passing arbitrary JS args into named methods (can follow once `Value` round-trip is solid)

## API

### Rust → JS

```ts
/** Run `main` in the tree-walker; return value + full event log. */
function run(source: string): RunResult;

type RunResult = {
  ok: boolean;
  value?: JsonValue;   // present when main returned (even if ok is true)
  events: EventJson[]; // events collected; may be non-empty on mid-run failure
  error?: string;      // present when ok === false
};
```

Semantics:

1. `Engine::from_source(source)` — on load failure → `{ ok: false, events: [], error }`.
2. `run_main()` — on success → `{ ok: true, value, events }`.
3. On runtime error after partial execution → `{ ok: false, events: <so far>, error }`.

### I/O (v1)

| Direction | Behavior |
|-----------|----------|
| Input | Only what appears in source (literals / locals). No stdin. |
| Output | `main` return value + `events`. No stdout / `cout`. |

Visualizer drives UX from events (`Step`, `VarAssign`, `ContainerMod`, …), not printed text.

## Serialization

- Add `Serialize` (and minimal helpers) for `Value`, `Slot`, `Event`, `Address` as needed.
- Prefer tagged JSON: `{ "kind": "Step", "span": { "start", "end" }, ... }`.
- Heap objects appear as handles (`{ "object": id }`), not deep graphs — same as runtime model.
- Use `serde-wasm-bindgen` (or `JsValue` from `serde_json`) for the return object.

## Build

```bash
# one-time: rustup target add wasm32-unknown-unknown
# wasm-pack build crates/wasm --target web
```

- Crate type: `cdylib` (+ `rlib` if useful for tests).
- Workspace member: `crates/wasm`.
- Feature or separate package so native `cargo test --workspace` does not require wasm tooling for other crates.

## Why this shape

Matches the product story already agreed: **browser gives C++ text → WASM executes subset → events for the UI**. Tree-walker first because it covers the supported subset and already emits the full event v1 set; VM/stepping can plug in later behind the same `RunResult` shape.
