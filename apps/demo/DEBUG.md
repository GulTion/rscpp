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

Stable testids: `seeker-root`, `seeker-scrubber`, `seeker-play`, `seeker-pause`, `seeker-speed`, `editor-pane`, `editor-run`, `editor-root`, `ds-pane`, `ds-root`, `ds-object-picker`, `ds-repr-select`, `fixture-select`.

## Fixture switcher

Use `[data-testid=fixture-select]` → `parent_tree`, then pick object `#0` and set representation to `tree` via `[data-testid=ds-repr-select]`.
