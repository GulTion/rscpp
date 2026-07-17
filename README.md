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

## Visualizer contract

Event and value JSON (what `Object.value` means, `Alloc.size` / `elems`, shadow-heap recipe):

→ **[`docs/events.md`](docs/events.md)**

## Develop

```bash
cargo test --workspace

# CLI phase dump
cargo run -p rscpp-pipeline -- examples/main.cpp

# Corpus error catalog (parse+sema, batched)
cargo run -p rscpp-corpus -- --dir testing --limit 50
# see docs/superpowers/specs/2026-07-16-rscpp-corpus-design.md

# Browser demo (visualizer frontend)
wasm-pack build crates/wasm --target web
pnpm install
pnpm dev
# → http://localhost:5173
# Full guide: apps/demo/README.md

# Legacy minimal WASM page (no visualizer UI)
# python3 -m http.server 8080   # from repo root
# open http://localhost:8080/examples/web/
```

Build details, size tuning, and wasm-opt troubleshooting: **[`docs/wasm-build.md`](docs/wasm-build.md)**

```js
import init, { run, run_method } from "./pkg/rscpp_wasm.js";
await init();
const { ok, value, events, error } = run("int main() { return 42; }");
// LeetCode style (no main):
const r2 = run_method(source, "Solution::twoSum", [[2, 7, 11, 15], 9]);
const r3 = run_method(source, "Solution::isValid", ["()[]{}"]);
// error: { message, span?: { start, end } }
```

CLI demos: `examples/two_sum.cpp`, `examples/valid_parentheses.cpp`, `examples/dfs.cpp`.

## Scope

Initially: the LeetCode-common C++ subset. Deferred until the core is stable: coroutines, modules, RTTI, exceptions, filesystem, threads, locale.
