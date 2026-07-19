# Dark mode (visualizer) — design

**Date:** 2026-07-19  
**Status:** approved (conversation); pending implementation plan  
**Scope:** Full visualizer — demo chrome, editor, seeker, ds-viewer, debugger

## Goals

- Support light and dark UI for the entire demo visualizer.
- Default to the OS preference (`prefers-color-scheme`).
- Allow a manual override (light / dark / system) persisted in `localStorage`.
- One token source so packages do not drift into mixed light/dark chrome.

## Non-goals

- Per-`mount*()` theme prop / theme API on packages (document-level theme only).
- Redesigning segment colors, access flashes, or brand identity beyond readable contrast.
- Shipping a separate dark stylesheet per package.

## Approach

**CSS custom properties + `data-theme` on `<html>`.**

1. Preference stored as `"system" | "light" | "dark"` under key `rscpp-theme` (default `"system"`).
2. Effective theme is the override, or else `window.matchMedia("(prefers-color-scheme: dark)")`.
3. Always set `document.documentElement.dataset.theme` to `"light"` or `"dark"` (the *effective* value) so selectors stay simple: `[data-theme="dark"] { … }`.
4. When preference is `"system"`, listen for OS scheme changes and update `data-theme`.
5. Demo toolbar toggle cycles **System → Light → Dark → System…** and shows the current preference (e.g. `Theme: System`).

## Tokens

Define light defaults on `:root` / `[data-theme="light"]` and dark overrides under `[data-theme="dark"]`. Minimum set:

| Token | Role |
| --- | --- |
| `--bg` | Page / body background |
| `--panel` | Pane surfaces (editor, ds, seeker shells) |
| `--border` | Pane and control borders |
| `--text` | Primary text |
| `--muted` | Secondary / legend text |
| `--control-bg` | Buttons, inputs, selects |
| `--control-border` | Control borders |
| `--track` | Seeker track / slab base |
| `--thumb` | Playhead thumb (may stay high-contrast) |
| `--code-bg` / `--code-fg` | Editor surface (also fed into CodeMirror theme) |

Segment / highlight / access colors remain largely unchanged (semi-transparent accents already work on both backgrounds). Adjust only if contrast fails in dark.

## Package changes

| Area | Change |
| --- | --- |
| `apps/demo` | Theme bootstrap module, toggle in toolbar, `layout.css` uses tokens |
| `@rscpp/editor` | CodeMirror `EditorView.theme` (or dual themes) driven by effective `data-theme`; chip styles use tokens or dual-safe colors |
| `@rscpp/seeker` | Replace hardcoded light hex in mount UI with `var(--…)` |
| `@rscpp/ds-viewer` | Same for canvas chrome / pane chrome |
| `@debugger` | Same for overlay chrome |

Inline styles may use `var(--token)` — no requirement to move everything into stylesheets.

## Persistence & bootstrap

- Read `localStorage` as early as practical in demo `main.ts` (before first paint of themed surfaces if possible) to avoid a light flash on dark OS.
- Invalid / missing values → `"system"`.

## Testing

- Unit-test preference → effective theme resolution (system + mock `matchMedia`, light, dark).
- Manual smoke: toggle cycles, reload preserves preference, OS change updates when preference is System; editor/seeker/ds/debugger remain readable in both modes.

## Out of scope (explicit)

- Editor span “travel” highlight animation (removed; not part of this work).
- Theming `examples/web` or non-demo surfaces unless trivial.
