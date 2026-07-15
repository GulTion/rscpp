# rscpp

A C++ interpreter/runtime in idiomatic Rust, targeting WebAssembly. Built to eventually power an interactive algorithm visualizer for LeetCode-style C++.

JSCPP (sibling repo) is a **semantics reference only** — not a line-by-line port.

## Pipeline

```
source → preprocessor → lexer → parser → AST → sema → interpreter/VM → event stream
```

## Phases

| # | Crate | Status |
|---|-------|--------|
| 1 | `rscpp-lexer` | done |
| 2 | `rscpp-parser` | done |
| 3 | `rscpp-ast` | done |
| 4 | semantic analysis | planned |
| 5 | runtime | planned |
| 6 | STL | planned |
| 7 | memory model | planned |
| 8 | VM | planned |
| 9 | WASM bindings | planned |

## Develop

```bash
cargo test -p rscpp-lexer -p rscpp-ast -p rscpp-parser
```

## Scope

Initially: the LeetCode-common C++ subset. Deferred until the core is stable: coroutines, modules, RTTI, exceptions, filesystem, threads, locale.
