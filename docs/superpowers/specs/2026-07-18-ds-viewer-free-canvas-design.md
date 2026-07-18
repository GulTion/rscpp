# DS Viewer free canvas

Date: 2026-07-18  
Package: `@rscpp/ds-viewer`  
Status: approved for planning

## Problem

In **All live Allocs** mode, each heap object is a full-width card in a vertical stack with generous padding. Dense runs (`dfs_main`, etc.) waste space and force scrolling. Users want:

1. Panes sized to the data structure (minimal chrome/padding).
2. Free placement: drag each pane on a canvas.
3. Positions sticky by Alloc id across seek/play (same `#id` returns to the same place).

Seeker redesign is out of scope.

## Goals

- Content-sized object panes (shrink-wrap; no forced full width).
- Scrollable free canvas; drag via title-bar handle.
- `Map<objId, {x, y}>` persists for the life of a mounted viewer / timeline instance.
- Auto-place new ids; keep coords when an object leaves and later reappears.
- **Reset layout** control to clear positions and re-pack.

## Non-goals (v1)

- Persist layout across page reloads or `localStorage`.
- Snap-to-grid, magnetic align, or multi-select.
- Drag libraries (`interact.js`, dnd-kit, etc.).
- Changing graph/tree SVG internals beyond fitting inside a compact pane.
- New required public props (optional `getLayout` / `setLayout` later).

## Approach

**Absolute positioning on a relative canvas** (vanilla pointer events). Rejected: CSS-transform-only (no benefit), drag libraries (dependency for little gain).

## Layout & sizing

- `body` becomes a scrollable **canvas**: `position: relative`, fills available height, `overflow: auto`.
- Each object **pane**:
  - `position: absolute; left/top` from the position map (or auto-pack).
  - Width/height from content (`width: max-content` / intrinsic), not `100%`.
  - Chrome: ~2px padding, 1px border; compact header (monospace `#id type` + per-object repr `<select>`).
  - Inner views that are wide (long arrays, large matrices) scroll or wrap **inside** the pane; they must not force the pane to span the whole DS column.
- Vertical stack + `gap: 12px` + `padding: 8px` card style is removed for `mode === "all"` (and for single-object mode on the same canvas).

## Drag & positions

- **Handle**: title/header bar only. Repr select and view content do not start a drag.
- **Pointer events**: `pointerdown` on handle → `setPointerCapture` → move updates `left`/`top` → `pointerup` commits to the map. Raise `z-index` while dragging.
- **Store**: `positions: Map<number, { x: number; y: number }>` (CSS px relative to canvas).
  - Survives tick/seek/play while the same `Timeline` is mounted.
  - Cleared on `destroy()`, when `update` swaps in a **different** timeline instance (new Run), or when the user clicks **Reset layout**.
- **First appearance**: if id has no entry, auto-pack after measure (simple row/column slots with ~8px gap; avoid obvious overlap with existing positions when cheap).
- **Absent from snapshot**: remove the DOM pane but **keep** map entry; on return, reuse `{x,y}`.
- **Toolbar**: existing mode/repr controls + **Reset layout** button.

## Paint / lifecycle

- On tick/seek: show only live Allocs; for each id apply saved position if present, else auto-pack once and store.
- Do not reset `left`/`top` for ids that already have positions.
- `mode === "single"`: one pane on the same canvas (draggable); use the same map. If no position, place at a default (e.g. near top-left) rather than a separate layout system.
- Diff/highlight behavior inside views is unchanged.

## API

No required prop changes for v1.

```ts
// Unchanged surface
mountDsViewer(el, { timeline, objId, mode?, representation?, onRepresentationChange? })
```

Optional later (not in v1): `getLayout(): Record<number, {x,y}>` / `setLayout(...)`.

## Testing

- Unit or DOM smoke: after placing id `3` at `{x,y}`, seek away (object gone) and back → pane at same `{x,y}`.
- New Alloc without a map entry gets a position distinct from an existing pane (auto-pack).
- Reset layout clears map and re-packs.
- Drag handle moves pane; changing repr does not clear position.

## Acceptance

- Live Alloc panes are compact and content-sized.
- User can drag panes freely; positions stick by id across scrubbing.
- Reset layout restores an auto-packed arrangement.
- Demo `dfs_main` (multiple live objects) is usable without a tall padded stack.
