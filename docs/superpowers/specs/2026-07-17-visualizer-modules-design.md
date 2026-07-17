# Visualizer Modules Design

**Date:** 2026-07-17  
**Status:** approved (approach: layered packages)  
**Related:** [`docs/events.md`](../../events.md), [`docs/wasm-build.md`](../../wasm-build.md)

## Problem

rscpp already emits a stable event stream from WASM (`run` / `run_method`). We need reusable visualization modules — Editor, DS Viewer, Seeker — that any site or extension can glue in, not a one-off LeetCode UI.

## Goals

- **Modules first, hosts later.** Demo site is the lab; deep LeetCode DOM blend comes after modules are solid.
- **Local-only execution** when using the runner (WASM in-browser / Node).
- **Headless timeline core** so non-React / non-DOM hosts can drive playhead + heap state.
- **One Editor core**, two run profiles: LeetCode (`Solution` + args/expected) and Codeforces (`main`; I/O panel stub until stdin exists).
- **Representation engine** for DS views: infer candidates from heap shape; user picks; linear views polished in v1; graph/tree usable.

## Non-goals (v1)

- Deep LeetCode page injection / side-panel blend
- Real Codeforces stdin/stdout (rscpp has no console I/O yet)
- Collaborative editing, multi-file projects
- Pixel-perfect graph layout research
- Shipping as a browser extension in the first cut

## Decisions locked

| Topic | Choice |
|-------|--------|
| Packaging | Headless TS core + thin UI adapters |
| Run vs play | `@rscpp/runner` (WASM) + visualizer packages (events); both local; split for maintainability |
| First host | Demo site as lab |
| Editor modes | One Editor + LC / CF run profiles |
| DS Viewer | Representation engine; linear first; graph/tree usable |
| Code editor | CodeMirror 6 |
| Layout | Layered packages (not mega-package, not single UI kit) |

## Architecture

```text
apps/demo/                    # lab host — glues packages
packages/
  timeline/   @rscpp/timeline   pure TS, no DOM
  runner/     @rscpp/runner     WASM bind → RunResult
  editor/     @rscpp/editor     CodeMirror 6 + profiles
  ds-viewer/  @rscpp/ds-viewer  canvases + representation engine
  seeker/     @rscpp/seeker     scrubber / play UI
```

Dependency direction (downward only):

```text
demo → editor, ds-viewer, seeker, runner
editor, ds-viewer, seeker → timeline
runner → (rscpp WASM artifact; not timeline)
```

Hosts may inject a `RunResult` without `@rscpp/runner` (e.g. precomputed fixtures, future remote runner).

### Data flow

1. Host obtains `RunResult` via `runner.run` / `runner.runMethod`, or injects one.
2. Host creates a `Timeline` from `events` (+ source string for spans).
3. UI modules subscribe to timeline ticks; they never own the event log.
4. Playhead moves → timeline rebuilds heap / frame state → subscribers update.

## Package: `@rscpp/timeline`

**Role:** Pure playhead + reconstruction over `EventJson[]`.

### Public surface (conceptual)

```ts
type Timeline = {
  length: number;           // events.length
  index: number;            // playhead t ∈ [0, length]
  seek(t: number): void;
  step(delta: number): void;
  play(opts?: { speed?: number }): void;
  pause(): void;

  /** Heap + frames as of events[0..index) applied. */
  snapshot(): HeapSnapshot;

  subscribe(listener: (ev: TimelineEvent) => void): () => void;
};

type TimelineEvent =
  | { type: "tick"; index: number; snapshot: HeapSnapshot }
  | { type: "highlight"; ranges: HighlightRange[] }
  | { type: "seek"; index: number };

type HighlightRange = {
  start: number;
  end: number;
  kind: string;             // event kind or derived (write, loop, error, …)
};

type HeapSnapshot = {
  objects: Map<number, ObjectState>;  // ObjId → reconstructed object
  frames: FrameState[];               // from FnEnter/FnExit + Writes
  openLoops: number[];                // loop_id stack at playhead
};
```

### Reconstruction rules

- Apply events in order up to (not including) the event at `index`, or including — **pick including:** playhead points at “just after applying event `index-1`”; when `index === 0`, empty heap. Seeking to `t` means “state after the first `t` events.”
- Drive heap from `Alloc`, `Write`, `ContainerMod` (and related) per [`docs/events.md`](../../events.md).
- Drive frames from `FnEnter` / `FnExit` and local `Write`s.
- Group body events into open `loop_id` instances for seeker coloring (stack discipline from LoopIter / LoopEnd).

### Performance

- For scrubbing, prefer **incremental** apply/rewind when `|Δt|` is small; full replay from 0 when jumping far or when rewind is not yet implemented.
- v1 may full-replay on every seek if N is modest; document the upgrade path (checkpoints every K events).

### Testing

- Unit tests with fixture `RunResult` JSON: seek to known indices, assert object sizes / values / open loops.
- No DOM, no WASM in this package’s unit tests.

## Package: `@rscpp/runner`

**Role:** Thin WASM wrapper around existing `rscpp` exports.

```ts
run(source: string): Promise<RunResult>;
runMethod(source: string, method: string, args: unknown[]): Promise<RunResult>;
```

- Loads the wasm-pack artifact built from `crates/wasm`.
- Types mirror [`docs/events.md`](../../events.md) (`RunResult`, `EventJson`, `ValueJson`).
- Errors surface as `ok: false` + `error` from the engine; load failures throw or return a synthetic failed `RunResult` — **prefer** synthetic `{ ok: false, events: [], error }` for consistency with engine load failures.

## Package: `@rscpp/editor`

**Role:** CodeMirror 6 document + decorations + run-profile chrome.

### Run profiles

| Profile | Entry | Host UI extras |
|---------|-------|----------------|
| `leetcode` | `runMethod(source, method, args)` | method name, JSON args, optional expected |
| `codeforces` | `run(source)` | I/O panel **stub** (disabled / “coming when stdin lands”) |

Shared: source buffer, Run button, error banner from `RunResult.error` (message + span highlight).

### Visualizations

1. **Span decorations** — colors by event kind (write / alloc / container / loop / error / step). Source of ranges: timeline `highlight` (current event + optional hover).
2. **Inline value chips** — CodeMirror decoration widgets after identifier tokens that match recent local `Write`s in the current frame, e.g. `sum⦃5⦄`. Hover shows full `ValueJson` / `ObjId`. Missing name → no chip (do not invent).
3. **Bidirectional seek** — click a line → `timeline.seek` to nearest event whose span intersects that line; seeker hover → temporary editor highlight.

Editor does **not** own playhead; it only calls timeline APIs and paints from subscriptions.

## Package: `@rscpp/ds-viewer`

**Role:** Bind to one or more `ObjId`s + timeline; render via chosen representation.

### Representation engine

Given `ObjectState` at playhead `t`, propose ordered candidates:

| Shape heuristic | Candidates |
|-----------------|------------|
| sequence (`vector` / `deque` / `list` / `array`) of scalars | array, table, stack, queue |
| sequence of sequences / pairs | matrix, edge-list |
| `map` / adjacency-like | adjacency list, table |
| parent-index `vector<int>` | tree |
| fallback | raw table of slots |

User selects a representation; preference is remembered per `ObjId` for the session (in-memory map; optional `localStorage` later).

### v1 fidelity

- **Polished:** array, table, stack, queue (linear).
- **Usable:** graph (nodes/edges from edge-list or adjacency), tree (parent array or nested).
- Animation: **snapshot-diff morphs** — compare previous vs current object state on tick; animate insert / remove / move / recolor. No separate tween timeline beyond CSS/canvas transitions keyed off the diff.

### Object picker

Demo (and hosts) list objects from `Alloc` events; selecting one mounts a viewer pane.

## Package: `@rscpp/seeker`

**Role:** Video-like control over event index.

- Scrubber: `0 … timeline.length`.
- Play / pause / speed multiplier.
- **Segments** colored by open `loop_id` (and optional fn segments from call stack).
- Tooltip on hover: source line + short snippet (host supplies source or line map).
- Hover scrubber → timeline emits temporary highlight → editor paints.
- **Loop markers:** ticks from `LoopIter` (and optional `LoopEnd`).
- Large seeks: seeker calls `timeline.seek(t)`; timeline batches notifications so DS + editor update once per seek (or once per animation frame while playing).

## Demo app (`apps/demo`)

Layout:

- **Left:** Editor (profile toggle LC / CF).
- **Center:** DS Viewer + object picker.
- **Bottom:** Seeker.

Purpose: exercise package APIs, fixtures, and UX stress — not product branding.

Suggested fixtures: small LC-style `twoSum`, loop-heavy nest, vector mutations, parent-array tree.

## Error handling

| Case | Behavior |
|------|----------|
| Parse/runtime fail mid-run | Timeline still loads partial `events`; error banner + span; seeker max = events so far |
| Empty events | Seeker disabled; editor shows source only |
| Unknown object shape | DS falls back to raw table |
| WASM load fail | Runner returns failed `RunResult` or throws once at init — demo shows setup error |

## Phased delivery (one plan, staged)

1. **Scaffold** — monorepo packages + `@rscpp/timeline` + `@rscpp/runner` + demo shell with fixture JSON.
2. **Seeker + Editor basics** — scrub, play, span highlights, LC profile run via WASM.
3. **Inline chips + CF profile stub.**
4. **DS Viewer** — representation engine + linear views + diff animation.
5. **Graph/tree usable** + seeker loop coloring / tooltips polish.

## Out of scope reminders

CF real I/O, LeetCode deep blend, extension packaging, and checkpointed timeline rewind optimization can follow without changing the public package boundaries above.
