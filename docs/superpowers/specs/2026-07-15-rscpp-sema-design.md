# rscpp Semantic Analysis Design

**Date:** 2026-07-15  
**Crate:** `rscpp-sema`  
**Phase:** 4 (type checker / name resolution)

## Problem

JSCPP mostly discovers types at runtime via dynamic `VariableType` objects. The visualizer and interpreter need earlier, typed feedback: undefined names, bad assignments, and enough type info for `vector`/`pair` member calls.

## Goals

- Walk a `TranslationUnit` and report semantic errors with spans.
- Scoped name resolution (global / class / function / block).
- Resolve AST types to a compact `Ty` (builtins, pointers/refs, class/named templates).
- Light expression checking (arithmetic, compare, assign, call arity stubs).
- Seed known LeetCode STL templates (`vector`, `pair`, `map`, `string`, …) with common members (`size`, `push_back`, `[]`, `first`/`second`, …).

## Non-goals

- Full C++ overload resolution / conversion ranks
- Complete STL
- Template instantiation / metaprogramming
- Lifetime / borrow analysis

## API

```rust
pub fn analyze(tu: &TranslationUnit) -> SemaResult;

pub struct SemaResult {
    pub errors: Vec<SemaError>,
}
```

`errors` empty ⇒ program is semantically acceptable for the current subset.

## Why better than JSCPP

Typed scopes ahead of time; errors with spans before execution; STL stubs are explicit data, not JS monkey-patches on a runtime object bag.
