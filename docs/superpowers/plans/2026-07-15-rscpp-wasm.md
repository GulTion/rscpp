# WASM run() Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose `run(source) → { ok, value?, events, error? }` via WASM for the browser.

**Architecture:** New `rscpp-wasm` crate wraps tree-walker `Engine`. Serialize `Value`/`Event`/`Slot`/`Address`/`Span` with serde; return a `JsValue` via `serde-wasm-bindgen`. No I/O.

**Tech Stack:** wasm-bindgen, serde, serde-wasm-bindgen, wasm-bindgen-test (optional)

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-15-rscpp-wasm-design.md`
- Run-only; tree-walker only; no stdin/stdout; no stepper/VM exports
- Tag events as `{ "kind": "...", ... }`

---

## File map

| File | Role |
|------|------|
| `crates/lexer` | `Serialize` on `Span` (feature or direct serde dep) |
| `crates/runtime` | `Serialize` on `Value`, `Address`, `Slot`, `Event` |
| `crates/wasm/` | `cdylib`, `#[wasm_bindgen] pub fn run` |
| Workspace `Cargo.toml` | member + wasm may be excluded from default host tests if needed |

---

### Task 1: Serde on Span / Value / Event

- [x] Add `serde` (derive) to lexer + runtime
- [x] `#[derive(Serialize)]` + `#[serde(tag = "kind")]` on `Event` and `Slot`/`Value`/`Address` (use internally tagged or adjacently tagged for clear JS)
- [x] Unit test: serialize a small `Event::Step` to JSON string

### Task 2: `rscpp-wasm` crate

- [x] Scaffold `crates/wasm` with `cdylib`, deps: wasm-bindgen, serde, serde_json or serde-wasm-bindgen, rscpp-runtime
- [x] Implement `run(source: &str) -> JsValue` matching `RunResult`
- [x] Native/unit test of run result shape via serde_json (gate wasm-only bits with `cfg`)

### Task 3: Verify

- [x] `cargo test -p rscpp-runtime` and `-p rscpp-wasm` (host)
- [x] Optional: `wasm-pack build crates/wasm --target web` if tooling present

---
