# rscpp

[![Deploy demo](https://github.com/GulTion/rscpp/actions/workflows/pages.yml/badge.svg)](https://github.com/GulTion/rscpp/actions/workflows/pages.yml)
![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![WebAssembly](https://img.shields.io/badge/target-wasm32-654ff0?logo=webassembly)
![License: MIT](https://img.shields.io/badge/license-MIT-blue)

**A C++ interpreter written in Rust, compiled to WebAssembly, that lets you watch LeetCode-style C++ run step by step in the browser.**

**[Live demo → gultion.github.io/rscpp](https://gultion.github.io/rscpp/)**

rscpp runs C++ source directly without a compiler toolchain, and records every variable write, function call, loop iteration, and container change as a stream of events. The browser visualizer replays that stream so you can scrub back and forth through an algorithm and see the call tree, the heap, and data structures (arrays, stacks, graphs, trees) change over time.

---

## Features

- **Runs real C++**: classes, constructors with member-init lists, lambdas, `auto`, references, and `using` aliases.
- **LeetCode mode**: call `Solution::twoSum` with JSON arguments without writing a `main`.
- **Large STL subset**: `vector`, `deque`, `list`, `array`, `bitset`, `stack`, `queue`, `priority_queue`, `map` / `unordered_map`, `set` / `unordered_set`, `string`, and `pair`, plus most of `<algorithm>`, `<numeric>` and `<functional>`.
- **Execution events**: a documented JSON event stream (`VarAssign`, `FnEnter`, `ContainerMod`, `Alloc`, `Compare`, and more) that any frontend can consume.
- **Visual debugger**: a CodeMirror editor with live value chips, a timeline scrubber with play and speed controls, a function call tree, and data-structure panes.
- **Precise errors**: parse, semantic, and runtime errors point at source spans.
- **Small and portable**: about 500 KB of WASM (about 190 KB gzipped) with no server required.

## How it works

```text
C++ source → preprocessor → lexer → parser → AST → sema → interpreter / VM → event stream → visualizer
```

| Crate | Role |
|-------|------|
| `rscpp-lexer` | Tokens and lexical diagnostics |
| `rscpp-parser` / `rscpp-ast` | Recursive-descent parser producing a typed AST |
| `rscpp-sema` | Name resolution and type checking |
| `rscpp-runtime` | Tree-walking interpreter, memory model, and the STL implementation |
| `rscpp-vm` | Experimental bytecode VM (subset of the language) |
| `rscpp-wasm` | `wasm-bindgen` bindings: `run` and `run_method` |
| `rscpp-pipeline` | CLI that dumps every phase for a source file |
| `rscpp-corpus` | Batch runner that catalogs parse and sema errors over a corpus |

| Package (TypeScript) | Role |
|----------------------|------|
| `@rscpp/runner` | Loads the WASM engine and exposes `run` / `runMethod` |
| `@rscpp/timeline` | Playhead and heap reconstruction from events |
| `@rscpp/editor` | CodeMirror 6 with span highlights and value chips |
| `@rscpp/seeker` | Scrubber, play/pause, and speed control |
| `@rscpp/ds-viewer` | Data-structure panes and function call tree |
| `@rscpp/debugger` | Debugger chrome and panes |
| `apps/demo` | The Vite app that ties everything together |

## Quick start

### Prerequisites

- [Rust](https://rustup.rs/) (stable) with the WASM target: `rustup target add wasm32-unknown-unknown`
- [wasm-pack](https://github.com/drager/wasm-pack): `cargo install wasm-pack`
- Node.js 20+ and [pnpm](https://pnpm.io/) 9 (`corepack enable`)

### Run the visualizer locally

```bash
git clone https://github.com/GulTion/rscpp.git
cd rscpp

wasm-pack build crates/wasm --target web   # builds crates/wasm/pkg
pnpm install
pnpm dev                                   # http://localhost:5173
```

Pick a fixture (`two_sum`, `dfs`, `n_queens`, `valid_parentheses`) or write your own code, click **Run**, then scrub the timeline. **Space** toggles play/pause. See [`apps/demo/README.md`](apps/demo/README.md) for the full guide.

### Use the CLI

```bash
cargo run -p rscpp-pipeline -- examples/two_sum.cpp
```

This prints the output of each phase (tokens, AST, sema, interpreter events, VM). Other examples are in [`examples/`](examples/): `dfs.cpp`, `n_queens.cpp`, `valid_parentheses.cpp`, and `main.cpp`.

## Use the engine from JavaScript

```js
import init, { run, run_method } from "./pkg/rscpp_wasm.js";

await init();

// Program with main()
const { ok, value, events, error } = run("int main() { return 42; }");

// LeetCode style: no main, call a method with JSON arguments
const r = run_method(source, "Solution::twoSum", [[2, 7, 11, 15], 9]);

// On failure: error = { message, span?: { start, end } }
```

The shape of every event and value, plus a recipe for rebuilding the heap on the client, is documented in **[`docs/events.md`](docs/events.md)**.

## Deploying to GitHub Pages

Every push to `main` runs [`.github/workflows/pages.yml`](.github/workflows/pages.yml), which builds the WASM engine and the demo and then publishes `apps/demo/dist`. To enable it on a fork, set **Settings → Pages → Source** to **GitHub Actions**.

To build the static site yourself:

```bash
wasm-pack build crates/wasm --target web
pnpm --filter demo build     # output: apps/demo/dist
pnpm --filter demo preview   # serve it locally
```

Size tuning and `wasm-opt` troubleshooting are covered in [`docs/wasm-build.md`](docs/wasm-build.md).

## Development

```bash
cargo test --workspace        # Rust tests
pnpm test                     # TypeScript package tests
pnpm build                    # type-check all packages and build the demo

# Catalog parse/sema errors over a directory of C++ files
cargo run -p rscpp-corpus -- --dir testing --limit 50
```

## Scope and status

rscpp targets the C++ subset commonly used in LeetCode and competitive programming. [`SUPPORTED.md`](SUPPORTED.md) is the detailed, per-member support matrix.

**Not supported yet:** templates (STL types are built in), inheritance and virtual functions, exceptions, preprocessor macros, coroutines, modules, RTTI, filesystem, threads, and locale.

The interpreter follows [JSCPP](https://github.com/felixhao28/JSCPP) as a reference for semantics only; it is not a port.

## License

MIT
