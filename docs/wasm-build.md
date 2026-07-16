# WASM build

How to produce the browser package under `crates/wasm/pkg/`.

## Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (0.13+ works; 0.15+ recommended)

## Build

From the repo root:

```bash
wasm-pack build crates/wasm --target web
```

Output:

| File | Purpose |
|------|---------|
| `crates/wasm/pkg/rscpp_wasm.js` | JS glue (imports/exports) |
| `crates/wasm/pkg/rscpp_wasm_bg.wasm` | Optimized WASM binary |
| `crates/wasm/pkg/rscpp_wasm.d.ts` | TypeScript types |

Serve the demo:

```bash
python3 -m http.server 8080   # repo root
# open http://localhost:8080/examples/web/
```

## What wasm-pack does

`wasm-pack build` is not a single step. It runs:

```text
rustc (release)  →  raw .wasm
       ↓
wasm-bindgen     →  JS glue + browser-ready exports
       ↓
wasm-opt         →  smaller/faster .wasm for shipping
```

### rustc (release)

Workspace `Cargo.toml` uses a size-oriented release profile:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

This is the first size pass: LLVM optimizes for binary size, links once, strips symbols.

### wasm-bindgen

Post-processes the compiler output so the browser can call Rust:

- exports `run`, `run_method`, etc. to JavaScript
- marshals strings, JSON, and errors across the JS/WASM boundary
- generates `rscpp_wasm.js` alongside the `.wasm` file

wasm-bindgen is **interop**, not the main size optimizer.

### wasm-opt (`-Oz`)

Binaryen’s `wasm-opt` runs a **second optimization pass** on the final WASM:

- dead-code elimination
- instruction shrinking / inlining
- WASM-specific transforms rustc did not apply

`-Oz` means optimize aggressively for **file size** (good for browser download).

Configured in `crates/wasm/Cargo.toml`:

```toml
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz", "--enable-bulk-memory", "--enable-nontrapping-float-to-int"]
```

Typical sizes (approximate, machine-dependent):

| Stage | Artifact |
|-------|----------|
| `cargo build --target wasm32-unknown-unknown --release` | ~700 KB (`target/.../rscpp_wasm.wasm`) |
| `wasm-pack build` (with wasm-opt) | ~330 KB (`crates/wasm/pkg/rscpp_wasm_bg.wasm`) |

## Troubleshooting

### `wasm-opt` fails: bulk memory / table.fill

Rust 1.87+ emits WebAssembly bulk-memory instructions (`memory.copy`, `table.fill`, …). Older wasm-pack bundles ship a `wasm-opt` that does not enable those features by default.

**Fix (already in this repo):** the `[package.metadata.wasm-pack.profile.release]` block above.

**Workaround (skip optimization):**

```bash
wasm-pack build crates/wasm --target web --no-opt
```

Build succeeds but the `.wasm` file is larger.

### Dev build (faster iteration)

```bash
wasm-pack build crates/wasm --target web --dev
```

Skips release opts; useful while changing bindings, not for size measurements.

## Raw cargo build (optional)

To compile WASM without wasm-pack (no JS glue, no wasm-opt):

```bash
cargo build -p rscpp-wasm --target wasm32-unknown-unknown --release
ls -lh target/wasm32-unknown-unknown/release/rscpp_wasm.wasm
```

Use wasm-pack for anything consumed by `examples/web/`.

## JS API (quick reference)

```js
import init, { run, run_method } from "./pkg/rscpp_wasm.js";

await init();

const r1 = run("int main() { return 42; }");
// { ok, value, events, error?: { message, span? } }

const r2 = run_method(source, "twoSum", "[[2,7,11,15], 9]");
```

See [`README.md`](../README.md) and [`docs/events.md`](events.md) for the event contract.
