# rscpp visualizer demo (frontend)

Vite + TypeScript lab app that glues the visualizer packages:

| Package | Role |
|---------|------|
| `@rscpp/timeline` | Playhead + heap reconstruction from events |
| `@rscpp/runner` | WASM `run` / `runMethod` |
| `@rscpp/editor` | CodeMirror 6 + span highlights + value chips |
| `@rscpp/seeker` | Scrubber / play / speed |
| `@rscpp/ds-viewer` | Multi-Alloc DS panes (array, stack, graph, tree, …) |

## Prerequisites

| Tool | Notes |
|------|--------|
| **Node.js** | 20+ recommended (18+ may work) |
| **pnpm** | 9.x — `npm i -g pnpm@9` or [corepack](https://nodejs.org/api/corepack.html) `corepack enable` |
| **Rust** | For building WASM ([rustup](https://rustup.rs/)) |
| **wasm-pack** | `cargo install wasm-pack` |

One-time Rust target:

```bash
rustup target add wasm32-unknown-unknown
```

## Quick start

From the **repository root** (`rscpp/`):

```bash
# 1. Build the WASM engine (outputs crates/wasm/pkg/)
wasm-pack build crates/wasm --target web

# 2. Install JS workspace deps
pnpm install

# 3. Start the demo
pnpm dev
# same as: pnpm --filter demo dev
```

Open **http://localhost:5173**

> `crates/wasm/pkg/` is produced by wasm-pack and is gitignored inside that folder. You must run step 1 on a fresh clone (or whenever the Rust engine changes).

## Production build

```bash
wasm-pack build crates/wasm --target web   # if pkg is missing/stale
pnpm install
pnpm --filter demo build                   # → apps/demo/dist/
pnpm --filter demo preview                 # serve dist locally
```

## Using the demo

1. Pick a **Fixture** (e.g. `two_sum`, `dfs`, `valid_parentheses`) or edit code and click **Run**.
2. Use the **seeker** (bottom): scrub, Play/Pause, speed (`1`–`1000` events/s).
3. **Space** toggles play/pause (ignored while typing in the editor).
4. **DS pane** defaults to **All live Allocs**; switch to **Single object** to focus one heap id.
5. Editor shows span highlights and inline chips (`nums⦃[2,7,11,15]⦄`, `nums.size()⦃4⦄`).

### Profiles

| Profile | Entry |
|---------|--------|
| `leetcode` | `runMethod(source, method, args)` — method + JSON args |
| `codeforces` | `run(source)` — `main()`; stdin panel is a stub for now |

## Debug

Dev console:

```js
window.__rscppDebug.seek(10)
window.__rscppDebug.snapshot()
window.__rscppDebug.highlight()
window.__rscppDebug.timeline.index
```

More MCP / testid notes: [`DEBUG.md`](./DEBUG.md)

## Tests (packages)

```bash
pnpm --filter @rscpp/timeline test
pnpm --filter @rscpp/editor test
pnpm --filter @rscpp/ds-viewer test
pnpm --filter @rscpp/seeker test
# or all workspace tests:
pnpm test
```

## Layout

```text
apps/demo/           ← this app
packages/timeline/
packages/runner/
packages/editor/
packages/seeker/
packages/ds-viewer/
crates/wasm/pkg/     ← wasm-pack output (required at runtime)
```

## Related docs

- Event / value contract: [`docs/events.md`](../../docs/events.md)
- WASM build details: [`docs/wasm-build.md`](../../docs/wasm-build.md)
- Module design: [`docs/superpowers/specs/2026-07-17-visualizer-modules-design.md`](../../docs/superpowers/specs/2026-07-17-visualizer-modules-design.md)

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Demo loads but Run fails / “wasm not initialized” | Run `wasm-pack build crates/wasm --target web`, restart `pnpm dev` |
| `pnpm: command not found` | Install pnpm 9 (`npm i -g pnpm@9`) |
| Port 5173 in use | Vite will offer another port; check the terminal URL |
| Highlights look wrong after editing source | Click **Run** again so events match the current buffer |
