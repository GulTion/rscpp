# Demo debugging (console + Playwright MCP)

## Start

```bash
# from repo root — rebuild wasm if crates/wasm/pkg is missing
wasm-pack build crates/wasm --target web

pnpm --filter demo dev
# → http://localhost:5173
```

## Console

```js
window.__rscppDebug.seek(10)
window.__rscppDebug.snapshot()
window.__rscppDebug.highlight()
window.__rscppDebug.timeline.index
```

## Playwright MCP

1. `browser_navigate` → `http://localhost:5173`
2. `browser_snapshot` — look for `seeker-play`, `seeker-scrubber`
3. `browser_click` → `[data-testid=seeker-play]`
4. `browser_evaluate` → `() => window.__rscppDebug.timeline.index`
5. `browser_evaluate` → `() => [...window.__rscppDebug.snapshot().objects.keys()]`
6. `browser_console_messages` — check for errors

Stable testids: `seeker-root`, `seeker-scrubber`, `seeker-play`, `seeker-pause`, `seeker-speed`, `editor-pane`, `editor-run`, `editor-root`, `ds-pane`, `ds-root`, `ds-mode`, `ds-object-picker`, `ds-repr-select`, `ds-pane-title-*`, `fixture-select`.

## DS mode

- **All live Allocs** (default): every heap object at the playhead in stacked panes.
- **Single object**: use Object picker + representation select.

## Complex fixtures

| Fixture | Events | DS tip |
|---------|--------|--------|
| `dfs` | ~410 | Object `#6 vector` (adj) → representation `graph` |
| `dfs_main` | ~444 | Same graph via `main` (CF profile) |
| `valid_parentheses` | ~270 | Object `#2 stack` → `stack`; scrub to watch push/pop |
| `valid_parentheses_main` | ~213 | Full main with two isValid calls |
| `n_queens` | ~2292 | Backtracking DFS for n=4; watch nested `dfs` + loops |

Regenerate:

```bash
cargo run -p rscpp-wasm --example dump_complex_fixtures
```
