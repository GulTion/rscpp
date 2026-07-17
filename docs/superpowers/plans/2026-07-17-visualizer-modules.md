# Visualizer Modules Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a layered TypeScript monorepo — `@rscpp/timeline`, `@rscpp/runner`, `@rscpp/editor`, `@rscpp/ds-viewer`, `@rscpp/seeker`, plus `apps/demo` — that visualizes rscpp WASM event streams with scrubbing, span highlights, and DS representations.

**Architecture:** Pure `@rscpp/timeline` owns playhead + shadow-heap reconstruction. UI packages expose framework-agnostic `mount(el, props)`. Demo is Vite + vanilla TS and exposes `window.__rscppDebug` for console + Playwright MCP debugging.

**Tech Stack:** pnpm workspaces, TypeScript, Vitest, Vite, CodeMirror 6, d3-hierarchy + dagre (positions only), rscpp WASM via wasm-pack (`crates/wasm`), Playwright MCP for demo UX verification.

**Spec:** `docs/superpowers/specs/2026-07-17-visualizer-modules-design.md`  
**Event contract:** `docs/events.md` (shadow-heap recipe)

## Global Constraints

- Packages must not depend on React/Vue; only `mount` / headless APIs.
- Timeline has **no DOM** and **no WASM**.
- Runner load/engine failures → synthetic `{ ok: false, events: [], error: { message } }` (prefer over throw after init).
- Seek `t` = state **after the first `t` events** (`t === 0` → empty).
- Prefer `Write` over duplicate `VarAssign` when reconstructing locals (`docs/events.md`).
- Graph/tree: SVG + custom draw; layout libs for positions only.
- Every UI root gets stable `data-testid` for MCP.
- Demo exposes `window.__rscppDebug` in development.
- YAGNI: no LeetCode DOM blend, no real CF stdin, no React wrappers in v1.
- Each task: test → implement → green → commit.

---

## File map

| Area | Paths |
|------|--------|
| Workspace | `package.json`, `pnpm-workspace.yaml`, `tsconfig.base.json`, `.gitignore` (node_modules, dist) |
| Timeline | `packages/timeline/package.json`, `src/index.ts`, `src/types.ts`, `src/createTimeline.ts`, `src/reconstruct.ts`, `src/fixtures/*.json`, `tests/*.test.ts` |
| Runner | `packages/runner/package.json`, `src/index.ts`, `src/types.ts` |
| Editor | `packages/editor/package.json`, `src/index.ts`, `src/mount.ts`, `src/decorations.ts`, `src/chips.ts`, `src/profiles.ts` |
| Seeker | `packages/seeker/package.json`, `src/index.ts`, `src/mount.ts`, `src/segments.ts` |
| DS Viewer | `packages/ds-viewer/package.json`, `src/index.ts`, `src/mount.ts`, `src/represent.ts`, `src/views/{array,table,stack,queue,matrix,graph,tree,raw}.ts`, `src/diff.ts` |
| Demo | `apps/demo/package.json`, `index.html`, `src/main.ts`, `src/debug.ts`, `src/layout.css`, `public/fixtures/*.json` |
| Docs | `apps/demo/DEBUG.md` (MCP + `__rscppDebug` cookbook) |

---

### Task 1: Monorepo scaffold + Vitest

**Files:**
- Create: `package.json`, `pnpm-workspace.yaml`, `tsconfig.base.json`
- Create: `packages/timeline/package.json`, `packages/timeline/tsconfig.json`, `packages/timeline/src/index.ts` (placeholder export)
- Create: `apps/demo/package.json`, `apps/demo/index.html`, `apps/demo/src/main.ts`, `apps/demo/vite.config.ts`
- Modify: `.gitignore` — add `node_modules/`, `dist/`, `*.tsbuildinfo`

**Interfaces:**
- Produces: pnpm workspace with packages `@rscpp/timeline`, app `demo`

- [ ] **Step 1: Root workspace files**

```json
// package.json
{
  "name": "rscpp-js",
  "private": true,
  "packageManager": "pnpm@9.15.0",
  "scripts": {
    "test": "pnpm -r run test",
    "build": "pnpm -r run build",
    "dev": "pnpm --filter demo dev"
  }
}
```

```yaml
# pnpm-workspace.yaml
packages:
  - "packages/*"
  - "apps/*"
```

```json
// tsconfig.base.json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "declaration": true,
    "skipLibCheck": true,
    "esModuleInterop": true
  }
}
```

- [ ] **Step 2: Stub `@rscpp/timeline` + demo Vite app**

`packages/timeline` exports `export const TIMELINE_VERSION = 1;`  
`apps/demo` imports it and `document.body.textContent = String(TIMELINE_VERSION)`.

- [ ] **Step 3: Install and smoke**

```bash
pnpm install
pnpm --filter @rscpp/timeline exec tsc --noEmit
pnpm --filter demo build
```

Expected: exit 0.

- [ ] **Step 4: Commit**

```bash
git add package.json pnpm-workspace.yaml tsconfig.base.json packages/timeline apps/demo .gitignore
git commit -m "chore: scaffold pnpm workspace for visualizer packages"
```

---

### Task 2: `@rscpp/timeline` — types + createTimeline + reconstruction

**Files:**
- Create: `packages/timeline/src/types.ts`, `createTimeline.ts`, `reconstruct.ts`, `index.ts`
- Create: `packages/timeline/tests/reconstruct.test.ts`, `packages/timeline/tests/timeline.test.ts`
- Create: `packages/timeline/src/fixtures/vector_push.json` (minimal hand-written events)
- Modify: `packages/timeline/package.json` — vitest script

**Interfaces:**
- Produces:

```ts
// packages/timeline/src/types.ts (canonical)
export type Span = { start: number; end: number };
export type ValueJson =
  | { kind: "Void" }
  | { kind: "Bool"; value: boolean }
  | { kind: "Int"; value: number }
  | { kind: "Float"; value: number }
  | { kind: "Char"; value: string }
  | { kind: "Nullptr" }
  | { kind: "Object"; value: number }
  | { kind: "Str"; value: string }
  | { kind: "Ptr"; value: unknown }
  | { kind: "Ref"; value: unknown };

export type EventJson = { kind: string; span?: Span; [k: string]: unknown };

export type ObjectState = {
  type_name: string;
  elems?: ValueJson[];
  entries?: { key: unknown; value?: ValueJson }[];
};

export type FrameState = {
  call_id: number;
  name: string;
  parent_id: number | null;
  locals: Map<string, ValueJson>;
};

export type HeapSnapshot = {
  objects: Map<number, ObjectState>;
  frames: FrameState[];
  openLoops: number[];
};

export type HighlightRange = { start: number; end: number; kind: string };

export type TimelineEvent =
  | { type: "tick"; index: number; snapshot: HeapSnapshot }
  | { type: "highlight"; ranges: HighlightRange[] }
  | { type: "seek"; index: number };

export type Timeline = {
  readonly length: number;
  readonly index: number;
  readonly source: string;
  seek(t: number): void;
  step(delta: number): void;
  play(opts?: { speed?: number }): void;
  pause(): void;
  snapshot(): HeapSnapshot;
  highlight(): HighlightRange[];
  subscribe(listener: (ev: TimelineEvent) => void): () => void;
};

export function createTimeline(opts: {
  events: EventJson[];
  source?: string;
}): Timeline;

export function reconstruct(events: EventJson[], t: number): HeapSnapshot;
```

- [ ] **Step 1: Write failing tests**

```ts
// packages/timeline/tests/reconstruct.test.ts
import { describe, it, expect } from "vitest";
import { reconstruct } from "../src/reconstruct";
import fixture from "../src/fixtures/vector_push.json";

describe("reconstruct", () => {
  it("t=0 is empty", () => {
    const s = reconstruct(fixture.events, 0);
    expect(s.objects.size).toBe(0);
  });

  it("after Alloc+push sees elems", () => {
    const s = reconstruct(fixture.events, fixture.events.length);
    const v = s.objects.get(0);
    expect(v?.type_name).toBe("vector");
    expect(v?.elems?.map((e) => (e as { value: number }).value)).toEqual([1, 2]);
  });
});
```

Hand-write `vector_push.json` with: `Alloc` id=0 vector size=0 elems=[], then two `ContainerMod` `push_back` (or `Write` Index) so final elems are `[1,2]`. Follow `docs/events.md` field names.

```ts
// packages/timeline/tests/timeline.test.ts
import { createTimeline } from "../src/createTimeline";
import fixture from "../src/fixtures/vector_push.json";

it("seek emits tick with matching snapshot", () => {
  const tl = createTimeline({ events: fixture.events, source: "" });
  const ticks: number[] = [];
  tl.subscribe((e) => {
    if (e.type === "tick") ticks.push(e.index);
  });
  tl.seek(fixture.events.length);
  expect(tl.index).toBe(fixture.events.length);
  expect(tl.snapshot().objects.get(0)?.elems?.length).toBe(2);
  expect(ticks.at(-1)).toBe(fixture.events.length);
});
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
pnpm --filter @rscpp/timeline test
```

- [ ] **Step 3: Implement `reconstruct` per shadow-heap recipe**

Handle at minimum: `Alloc`, `Dealloc`, `Write` (Local / Index / MapEntry / Object), `ContainerMod` (with and without `elems`), `FnEnter` / `FnExit`, `LoopIter` / `LoopEnd` / `Break` / `Continue`, `VarCreate` (optional if Write covers locals — prefer Write; still apply VarCreate if no prior Write for that name).

v1 seek: **full replay from 0** every time (document checkpoint TODO in a one-line comment only if needed — no separate TODO file).

- [ ] **Step 4: Implement `createTimeline`**

- Clamp seek to `[0, length]`
- `play`: `setInterval` / `requestAnimationFrame` stepping `index++` at `speed` events/sec (default 30); `pause` clears timer
- On seek/step/play tick: notify `{ type: "tick" }` and `{ type: "seek" }`; set highlight from event at `index-1` (or empty at 0) via `{ type: "highlight", ranges }`
- `highlight()` returns current ranges from last event’s span + kind

- [ ] **Step 5: Tests PASS + commit**

```bash
pnpm --filter @rscpp/timeline test
git add packages/timeline
git commit -m "feat(timeline): reconstruct heap and playhead API"
```

---

### Task 3: `@rscpp/runner` + wire WASM into demo fixtures path

**Files:**
- Create: `packages/runner/package.json`, `src/index.ts`, `src/types.ts`
- Create: `packages/runner/tests/run_smoke.test.ts` (optional skip if no wasm in CI — prefer generate fixture via native and commit JSON)
- Modify: `apps/demo` to load a committed fixture OR call runner when wasm present
- Create: `packages/timeline/src/fixtures/two_sum.json` — generate once with wasm or Rust, commit

**Interfaces:**
- Produces:

```ts
export type RunError = { message: string; span?: { start: number; end: number } };
export type RunResult = {
  ok: boolean;
  value?: unknown;
  events: import("@rscpp/timeline").EventJson[];
  error?: RunError;
};

export function initRunner(wasmUrl?: string): Promise<void>;
export function run(source: string): Promise<RunResult>;
export function runMethod(
  source: string,
  method: string,
  args: unknown[],
): Promise<RunResult>;
```

- [ ] **Step 1: Build wasm artifact**

```bash
wasm-pack build crates/wasm --target web
```

Expected: `crates/wasm/pkg/rscpp_wasm.js` exists.

- [ ] **Step 2: Implement runner wrapping pkg**

Import from a path relative to repo: either copy/symlink pkg into `packages/runner/wasm` during build script, or Vite alias `rscpp-wasm` → `crates/wasm/pkg`. Prefer Vite alias in demo + runner `initRunner` that dynamic-imports the glue and calls `await init(wasmUrl)`.

Map JS return object to `RunResult`. On init failure, later `run` returns `{ ok: false, events: [], error: { message: "wasm not initialized: …" } }`.

- [ ] **Step 3: Generate and commit `two_sum.json`**

Use `examples/two_sum.cpp` (or equivalent) via a tiny Node script calling runner, **or** Rust test dumping JSON. Commit events + source string in fixture.

- [ ] **Step 4: Vitest smoke (Node)**

If wasm works in Node with `--experimental-wasm-modules` / vite-node: assert `runMethod` returns `ok` and `events.length > 0`. If flaky in CI, mark test `it.skipIf(!process.env.RSCPP_WASM)` and always keep committed fixture tests in timeline.

- [ ] **Step 5: Commit**

```bash
git add packages/runner packages/timeline/src/fixtures apps/demo
git commit -m "feat(runner): wrap rscpp WASM run/runMethod"
```

---

### Task 4: `@rscpp/seeker` mount + demo shell layout

**Files:**
- Create: `packages/seeker/src/mount.ts`, `index.ts`, `segments.ts`
- Create: `packages/seeker/tests/segments.test.ts`
- Modify: `apps/demo/src/main.ts`, `src/layout.css`, `src/debug.ts`
- Create: `apps/demo/DEBUG.md`

**Interfaces:**
- Produces:

```ts
export type SeekerProps = {
  timeline: Timeline;
  source: string;
};

export function mountSeeker(el: HTMLElement, props: SeekerProps): MountHandle;
```

`data-testid`s: `seeker-root`, `seeker-scrubber`, `seeker-play`, `seeker-pause`, `seeker-speed`

- [ ] **Step 1: Unit test segment extraction**

```ts
// From LoopIter/LoopEnd events, build { loop_id, startIndex, endIndex }[]
it("pairs loop segments", () => {
  const segs = buildLoopSegments(events);
  expect(segs[0]).toMatchObject({ loop_id: 1, startIndex: 3 });
});
```

- [ ] **Step 2: Implement mountSeeker**

- Range input bound to `timeline.index` / `length`
- Play/pause/speed buttons call timeline APIs
- On input scrub: `timeline.seek(+value)`
- Hover scrubber: temporary highlight via timeline subscribe path — call optional `timeline` highlight override if you add `previewHighlight(ranges)` **or** emit by seeking without committing — **prefer** add `timeline.setHoverHighlight(ranges | null)` that notifies highlight without changing index

Add to Timeline in this task if missing:

```ts
setHoverHighlight(ranges: HighlightRange[] | null): void;
```

- Color track segments by `loop_id` (CSS gradient or absolute divs)
- Tooltip: map event span → line via `source` newline scan

- [ ] **Step 3: Demo layout + `__rscppDebug`**

```ts
// apps/demo/src/debug.ts
export function installDebug(api: RscppDebug) {
  (window as unknown as { __rscppDebug: RscppDebug }).__rscppDebug = api;
}
```

Wire: load fixture → `createTimeline` → `mountSeeker` → installDebug.

- [ ] **Step 4: Write `apps/demo/DEBUG.md`**

Document:
1. `pnpm --filter demo dev` → URL
2. Playwright MCP: `browser_navigate`, `browser_snapshot`, click `[data-testid=seeker-play]`
3. `browser_evaluate`: `() => window.__rscppDebug.snapshot()`
4. Console: `window.__rscppDebug.seek(10)`

- [ ] **Step 5: Manual/MCP verify + commit**

```bash
pnpm --filter demo dev
# MCP: navigate, snapshot, click play, evaluate snapshot().objects.size
git add packages/seeker apps/demo packages/timeline
git commit -m "feat(seeker): scrubber mount and demo debug handle"
```

---

### Task 5: `@rscpp/editor` — CodeMirror + span decorations + LC profile

**Files:**
- Create: `packages/editor/src/mount.ts`, `decorations.ts`, `profiles.ts`, `index.ts`
- Modify: `apps/demo/src/main.ts`

**Interfaces:**
- Produces:

```ts
export type EditorProfile = "leetcode" | "codeforces";

export type EditorProps = {
  timeline: Timeline;
  source: string;
  profile: EditorProfile;
  method?: string;           // default "Solution::twoSum"
  argsJson?: string;         // default "[[2,7,11,15],9]"
  expectedJson?: string;
  onRun?: (result: RunResult) => void;
  runMain?: (source: string) => Promise<RunResult>;
  runMethod?: (source: string, method: string, args: unknown[]) => Promise<RunResult>;
};

export function mountEditor(el: HTMLElement, props: EditorProps): MountHandle;
```

`data-testid`s: `editor-root`, `editor-run`, `editor-profile`, `editor-args`, `editor-error`

- [ ] **Step 1: Mount CodeMirror 6 with C++-ish language (or plain text)**

Use `@codemirror/lang-cpp` if available; else `EditorState` plain. Document must be editable; `update({ source })` resets doc when host reloads fixture.

- [ ] **Step 2: Span decorations from timeline highlights**

Map `HighlightRange.kind` → colors (write=amber, alloc=green, loop=blue, error=red, step=gray). Use CodeMirror `Decoration.mark` + `ViewPlugin` subscribed to timeline.

- [ ] **Step 3: Run profiles**

- `leetcode`: Run → parse `argsJson` → `runMethod(source, method, args)` → `onRun(result)`; show error banner from `result.error`
- `codeforces`: Run → `runMain(source)`; I/O panel is a disabled textarea with label `stdin (coming soon)`

- [ ] **Step 4: Click line → seek**

On gutter/line click: find nearest event whose span overlaps that line’s byte range; `timeline.seek(i+1)` (state after that event).

- [ ] **Step 5: Demo wire + MCP check + commit**

MCP: change scrubber → assert editor decoration present via snapshot; click Run with fixture source using runner.

```bash
git add packages/editor apps/demo
git commit -m "feat(editor): CodeMirror mount, highlights, LC/CF profiles"
```

---

### Task 6: Editor inline value chips

**Files:**
- Create: `packages/editor/src/chips.ts`
- Modify: `packages/editor/src/mount.ts`
- Create: `packages/editor/tests/chips.test.ts` (pure: given locals map + doc positions heuristic)

**Interfaces:**
- Produces: widgets after identifier tokens matching `snapshot().frames.at(-1)?.locals` keys

- [ ] **Step 1: Pure formatter test**

```ts
expect(formatChip({ kind: "Int", value: 5 })).toBe("⦃5⦄");
expect(formatChip({ kind: "Object", value: 3 })).toBe("⦃#3⦄");
```

- [ ] **Step 2: Implement widget decorations**

Scan visible viewport lines for `\b(name)\b` where `name` ∈ current frame locals; place `Decoration.widget` after match. Skip if name not in locals. Hover title = `JSON.stringify(value)`.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(editor): inline live value chips from frame locals"
```

---

### Task 7: `@rscpp/ds-viewer` — representation engine + linear views

**Files:**
- Create: `packages/ds-viewer/src/represent.ts`, `diff.ts`, `mount.ts`, `views/array.ts`, `table.ts`, `stack.ts`, `queue.ts`, `raw.ts`, `index.ts`
- Create: `packages/ds-viewer/tests/represent.test.ts`, `diff.test.ts`
- Modify: `apps/demo` — object picker + mount DS

**Interfaces:**
- Produces:

```ts
export type Representation =
  | "array" | "table" | "stack" | "queue" | "matrix"
  | "edge-list" | "adjacency" | "tree" | "graph" | "raw";

export function proposeRepresentations(obj: ObjectState, heap: HeapSnapshot): Representation[];

export type DsViewerProps = {
  timeline: Timeline;
  objId: number | null;
  representation?: Representation;
  onRepresentationChange?: (r: Representation) => void;
};

export function mountDsViewer(el: HTMLElement, props: DsViewerProps): MountHandle;
```

`data-testid`s: `ds-root`, `ds-repr-select`, `ds-object-picker`

- [ ] **Step 1: Failing represent tests**

```ts
it("vector of ints → array/table/stack/queue", () => {
  const obj = { type_name: "vector", elems: [{ kind: "Int", value: 1 }] };
  expect(proposeRepresentations(obj, emptyHeap)).toContain("array");
});

it("vector of Object rows → matrix candidate", () => {
  // outer elems are Object ids; heap has child vectors
});
```

- [ ] **Step 2: Implement propose + linear DOM views**

Update on timeline tick: `diff(prev, next)` → patch cells (add/remove/text) with CSS transition class `ds-flash` for changed indices.

Remember representation per `objId` in a `Map` inside mount closure.

- [ ] **Step 3: Demo object picker**

List Alloc ids from `lastRun.events` (or scan timeline events once); selecting sets `objId`.

- [ ] **Step 4: MCP verify + commit**

MCP: pick object → assert `[data-testid=ds-root]` shows cells; seek → cell text changes; `browser_evaluate` snapshot objects.

```bash
git commit -m "feat(ds-viewer): representation engine and linear views"
```

---

### Task 8: Graph + tree SVG views

**Files:**
- Create: `packages/ds-viewer/src/views/graph.ts`, `tree.ts`, `matrix.ts`
- Create: `packages/ds-viewer/tests/layout_helpers.test.ts`
- Create: fixture `packages/timeline/src/fixtures/parent_tree.json` (parent array vector)

**Interfaces:**
- Consumes: `d3-hierarchy` for tree; `dagre` for directed graph from edge-list / adjacency
- Produces: SVG under `ds-root` with nodes `data-testid="ds-node-${id}"`

- [ ] **Step 1: Heuristics**

- `vector<int>` length n with values in `-1..n-1` → propose `tree` (parent array)
- `vector<vector<int>>` or `vector<pair>` → `edge-list` / `graph`
- `map` of vectors → `adjacency`

- [ ] **Step 2: SVG render + snapshot-diff morph**

On tick: recompute layout positions; animate `transform`/`cx`/`cy` for existing nodes; fade in new; fade out removed. If `|V| > 200`, show banner and fall back to `table`.

- [ ] **Step 3: MCP + commit**

```bash
git commit -m "feat(ds-viewer): SVG graph and tree representations"
```

---

### Task 9: Seeker polish + end-to-end demo pass (MCP)

**Files:**
- Modify: `packages/seeker/src/mount.ts`, `segments.ts`
- Modify: `apps/demo/src/main.ts` — fixture switcher (twoSum, loops, tree)
- Modify: `apps/demo/DEBUG.md` — full checklist

- [ ] **Step 1: Loop coloring + LoopIter markers on scrubber**

- [ ] **Step 2: Hover seeker ↔ editor highlight (already partially in Task 4 — verify)**

- [ ] **Step 3: Full MCP checklist (agent must run)**

With `pnpm --filter demo dev` running:

1. `browser_navigate` → demo URL  
2. `browser_snapshot` — see editor, seeker, ds  
3. Click `editor-run` (LC profile) — events load  
4. Click `seeker-play` — index advances (`__rscppDebug.timeline.index`)  
5. Scrub to mid — editor highlight + ds cells update  
6. Select tree fixture + graph/tree repr — nodes visible  
7. `browser_console_messages` — no unexpected errors  

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(demo): fixture switcher and seeker/MCP polish"
```

---

## MCP debugging playbook (all tasks)

| Step | Tool | Purpose |
|------|------|---------|
| Start | Shell: `pnpm --filter demo dev` | Lab server |
| Open | Playwright MCP `browser_navigate` | Load demo |
| Structure | `browser_snapshot` | Roles/testids |
| Act | `browser_click` / `browser_fill_form` | Run, scrub, pick object |
| State | `browser_evaluate` → `__rscppDebug.*` | Heap/playhead truth |
| Logs | `browser_console_messages` | Runtime errors |
| Visual | `browser_take_screenshot` | Only when layout/SVG disputed |

Do **not** use MCP instead of Vitest for reconstruct/represent logic.

---

## Self-review (plan vs spec)

| Spec item | Task |
|-----------|------|
| Layered packages | 1–8 |
| Timeline headless reconstruct | 2 |
| Runner WASM | 3 |
| Seeker scrub/play/loop | 4, 9 |
| Editor CM6 + profiles | 5 |
| Inline chips | 6 |
| DS represent + linear | 7 |
| SVG graph/tree | 8 |
| Demo lab layout | 4–9 |
| `mount` APIs | 4–8 |
| `__rscppDebug` + MCP | 4, 9 + playbook |
| CF I/O stub | 5 |
| Non-goals respected | Global constraints |

No TBD placeholders in task steps. Types for Timeline/`mount` are consistent across tasks (`setHoverHighlight` introduced in Task 4).
