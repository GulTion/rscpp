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
| 4 | `rscpp-sema` | done |
| 5 | `rscpp-runtime` | done |
| 6 | STL (in runtime) | done |
| 7 | memory model | done |
| 8 | `rscpp-vm` | done |
| 9 | `rscpp-wasm` | done |

## Develop

```bash
cargo test --workspace

# CLI phase dump
cargo run -p rscpp-pipeline -- examples/main.cpp

# Browser demo
wasm-pack build crates/wasm --target web
python3 -m http.server 8080   # from repo root
# open http://localhost:8080/examples/web/
```

```js
import init, { run } from "./pkg/rscpp_wasm.js";
await init();
const { ok, value, events, error } = run("int main() { return 42; }");
```

## Scope

Initially: the LeetCode-common C++ subset. Deferred until the core is stable: coroutines, modules, RTTI, exceptions, filesystem, threads, locale.
