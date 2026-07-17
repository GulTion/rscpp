# Loop lifecycle events (Break / Continue / LoopEnd + loop_id)

**Date:** 2026-07-17  
**Status:** approved (brainstorm)  
**Related:** `docs/events.md`, `crates/runtime/src/event.rs`

## Goal

Give visualizers enough structure to:

- Highlight `break` / `continue`
- Know when a loop **instance** starts and finishes
- Group body events that belong to the same loop **run** (including nested loops)

without stamping `loop_id` on every heap/var event.

## Decisions

| Topic | Choice |
|--------|--------|
| `loop_id` identity | **Runtime instance** (monotonic), like `call_id` |
| Same source loop re-entered later | **New** `loop_id` |
| Nested loops | **Different** `loop_id`s |
| Zero iterations | **No** loop events (never entered) |
| `LoopEnter` | **Not** emitted; first `LoopIter` opens the instance |
| `loop_id` on other events | **No** — only loop control events carry it |
| Break / Continue | Dedicated events **and** `LoopEnd` when the instance ends |
| Exit reasons | `exhausted` \| `break` \| `return` |

## Event shapes

All include optional `call_id` (current activation) and `span` (keyword / loop header as appropriate).

### `LoopIter` (extended)

Emitted immediately before each body execution (existing behavior).

```json
{
  "kind": "LoopIter",
  "call_id": 0,
  "loop_id": 5,
  "span": { "start": 0, "end": 0 }
}
```

- First `LoopIter` for an instance **mints** `loop_id` and opens that loop group.
- Further iters of the same instance reuse the same `loop_id`.

### `Continue` (new)

```json
{
  "kind": "Continue",
  "call_id": 0,
  "loop_id": 5,
  "span": { "start": 0, "end": 0 }
}
```

- Loop instance stays **open**.
- Next body run (if any) is another `LoopIter` with the same `loop_id`.

### `Break` (new)

```json
{
  "kind": "Break",
  "call_id": 0,
  "loop_id": 5,
  "span": { "start": 0, "end": 0 }
}
```

- Followed by `LoopEnd` with `reason: "break"` for that `loop_id`.

### `LoopEnd` (new)

```json
{
  "kind": "LoopEnd",
  "call_id": 0,
  "loop_id": 5,
  "reason": "exhausted",
  "span": { "start": 0, "end": 0 }
}
```

| `reason` | When |
|----------|------|
| `exhausted` | Condition false / range finished / normal fall-out of the loop |
| `break` | After `Break` for this instance |
| `return` | Function returns while this loop instance is still open (unwind); emit `LoopEnd` for each open loop from innermost to outermost |

`LoopEnd` is emitted **only** if the instance was opened (at least one `LoopIter`). Zero-iteration loops emit nothing.

## Sequencing examples

### Normal

```
LoopIter(loop_id=5)
… body …
LoopIter(loop_id=5)
… body …
LoopEnd(loop_id=5, reason="exhausted")
```

### Continue

```
LoopIter(loop_id=5)
Continue(loop_id=5)
LoopIter(loop_id=5)
LoopEnd(loop_id=5, reason="exhausted")
```

### Break

```
LoopIter(loop_id=5)
Break(loop_id=5)
LoopEnd(loop_id=5, reason="break")
```

### Nested + re-entry of inner

```
LoopIter(loop_id=3)                    // outer instance
  LoopIter(loop_id=4) … LoopEnd(4, exhausted)   // first inner instance
  LoopIter(loop_id=5) … LoopEnd(5, exhausted)   // new inner instance next outer iter
LoopEnd(loop_id=3, reason="exhausted")
```

### Return from inside nested loops

```
LoopIter(3)
  LoopIter(4)
  … return …
  LoopEnd(4, reason="return")
LoopEnd(3, reason="return")
FnExit(...)
```

Order: close innermost open loop first, then outer, then `FnExit` (existing). Exact interleaving with `VarDestroy` / `Dealloc` on unwind should match current frame teardown order as closely as practical.

## UI grouping recipe

Maintain a stack of open `loop_id`s:

1. On `LoopIter` with a `loop_id` not on the stack → push (new instance). Same id already on stack → still the current innermost (another iteration).
2. Body events without `loop_id` belong to the **innermost** open loop (if any).
3. On `Continue` → stay in that loop; do not pop.
4. On `LoopEnd` → pop that `loop_id` (must be innermost if nestings are well-formed).

No need to retrofit `loop_id` onto `Write` / `VarCreate` / etc.

## Runtime notes (implementation later)

- Engine keeps `next_loop_id` and a stack of active loop frames `{ loop_id, … }` while executing `While` / `DoWhile` / `For` / `ForRange`.
- Mint id on first successful entry into the body for that activation of the statement.
- `break` / `continue` resolve to the innermost loop frame (C++ semantics; labeled break out of scope for v1 unless already supported).
- On `return`, before / while unwinding the function, emit `LoopEnd(reason="return")` for each still-open loop frame in that activation (innermost first).

## Out of scope

- Labeled `break` / `continue` targeting non-innermost loops
- `loop_id` on non-loop events
- Changing `call_id` / `FnEnter` / `FnExit` semantics
- Emitting loop events for zero-iteration loops

## Docs / tests (when implementing)

- Update `docs/events.md` Stepping / control table.
- Runtime tests: nested for, continue, break, return-from-inner, empty range (no loop events).
