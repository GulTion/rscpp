# DS Viewer Free Canvas Implementation Plan

> **For agentic workers:** Implement task-by-task. Steps use checkbox syntax.

**Goal:** Content-sized, draggable Alloc panes on a free canvas with positions sticky by object id.

**Architecture:** Refactor `@rscpp/ds-viewer` `mount.ts`: replace vertical card stack with absolute panes on a scrollable canvas; `Map<id,{x,y}>` + title-bar pointer drag; auto-pack new ids; Reset layout.

**Tech Stack:** Vanilla DOM, TypeScript, Vitest (+ happy-dom for mount tests).

## Global Constraints

- Positions stick by Alloc id across seek; clear on destroy, new timeline, or Reset layout.
- No drag libraries; no localStorage persistence in v1.
- Shrink-wrap panes; minimal padding (~2px).

---

### Task 1: Canvas mount + positions + drag

**Files:** `packages/ds-viewer/src/mount.ts`, `packages/ds-viewer/vitest.config.ts`, `packages/ds-viewer/tests/canvas.test.ts`, `packages/ds-viewer/package.json` (happy-dom if needed)

- [ ] Implement canvas body, compact panes, position map, auto-pack, drag handle, Reset layout
- [ ] Tests: position survives seek away/back; reset clears; new id auto-packs
- [ ] `pnpm --filter @rscpp/ds-viewer test` green
- [ ] Commit
