# rscpp visualizer contract — values & events

**Audience:** anyone building a UI on `run(source)` / the event log.  
**Source of truth for shapes:** this file. Rust types in `rscpp-runtime` serialize to match it.

Related code: `crates/runtime/src/value.rs`, `crates/runtime/src/event.rs`, `crates/wasm`.

---

## `run()` result

```ts
type RunResult = {
  ok: boolean;
  value?: ValueJson;     // main's return value when execution finished
  events: EventJson[];   // full log (may be partial if ok === false mid-run)
  error?: {              // present when ok === false
    message: string;
    span?: { start: number; end: number };
  };
};
```

No stdin/stdout in v1 — reconstruct program state from **events** (and `value`).

### Entry points

| API | Behavior |
|-----|----------|
| `run(source)` | Parse + run `main()` |
| `run_method(source, method, args)` | Parse + call `method` (e.g. `"Solution::twoSum"`) with JSON args array. Arrays → `vector`, numbers → `int`, strings → heap `string`. |

`#include <…>` / `#pragma once` are skipped. Other preprocessor directives still error.

---

## Spans

Byte offsets into the **exact** source string passed to `run`:

```json
{ "start": 43, "end": 76 }
```

Half-open `[start, end)`. Use to highlight the statement / init that caused the event.

---

## Values (`ValueJson`)

Serde shape: `{ "kind": "...", "value": ... }` (no `value` field for unit-like kinds).

| `kind` | `value` field | Meaning |
|--------|---------------|---------|
| `Void` | — | no value |
| `Bool` | `boolean` | |
| `Int` | `number` (i64) | |
| `Float` | `number` | |
| `Char` | string (one char) | |
| `Nullptr` | — | |
| **`Object`** | **`number`** | **Heap object id** (`ObjId`). Not nested data. |
| `Ptr` | `AddressJson` | pointer |
| `Ref` | `AddressJson` | reference (alias) |

### Heap handles

```json
{ "kind": "Object", "value": 3 }
```

means “look up heap object **id `3`**.” There is **no** deep copy of the vector/map inside this value. To know size/contents at birth, use the matching **`Alloc`** for that id (see below). Later mutations use `Write` / `ContainerMod`.

### Addresses (`AddressJson`)

`{ "kind": "...", "value": ... }`:

| kind | value |
|------|--------|
| `Null` | — |
| `Stack` | `{ "frame": number, "name": string }` |
| `Heap` | `number` (object id) |
| `Index` | `{ "obj": number, "index": number }` |
| `Field` | `{ "obj": number, "field": string }` |
| `MapEntry` | `{ "obj": number, "key": ValueJson }` |

---

## Slots (`SlotJson`)

Where a write happened (for highlighting):

| `kind` | fields |
|--------|--------|
| `Local` | `name` |
| `Global` | `name` |
| `Object` | `obj` (heap id) |
| `Index` | `obj`, `index` |
| `MapEntry` | `obj`, `key` (same shape as `ValueJson` / `MapKey`: `{ "kind": "Int", "value": … }`, …) |
| `Field` | `obj`, `field` |

---

## Events (`EventJson`)

Every event has `"kind": "<Name>"` plus fields. Common: `span`.

### Stepping / control

| kind | fields | notes |
|------|--------|--------|
| `Step` | `span` | about to run a statement |
| `ScopeEnter` / `ScopeExit` | `span` | `{` / `}` |
| `Branch` | `then_taken`, `span` | `if` path |
| `LoopIter` | **`loop_id`**, `span` | start of a loop body iteration (mints `loop_id` on first iter of an instance) |
| `Continue` | **`loop_id`**, `span` | `continue;` — instance stays open |
| `Break` | **`loop_id`**, `span` | `break;` — followed by `LoopEnd` with `reason: "break"` |
| `LoopEnd` | **`loop_id`**, **`reason`**, `span` | instance finished; `reason`: `exhausted` \| `break` \| `return` |

`loop_id` is a **runtime instance** id (like `call_id`): nested loops differ; re-entering the same source loop later gets a new id. Zero-iteration loops emit **no** loop events. Only these loop events carry `loop_id` — group body events by the open-instance stack (see design spec).

Design: `docs/superpowers/specs/2026-07-17-loop-lifecycle-events-design.md`.

| `Compare` | `op`, `left`, `right`, `result`, `span` | |
| `FnEnter` | `name`, **`call_id`**, **`parent_id?`**, `args[]`, `span` | activation edge; **root/`main` has `call_id: 0`**, `parent_id: null`; `span` is callee body / def |
| `Call` | `name`, **`call_id`**, `args[]`, `span` | call expression about to run; **`span` is the call site**; precedes matching `FnEnter` with the same `call_id` |
| `FnExit` | `name`, **`call_id`**, **`parent_id?`**, `ret`, `span` | same ids as matching enter |

**`call_id` / `parent_id` appear on `FnEnter` / `FnExit` / `Call`.** Other events do not carry `call_id`. Attribute work to a frame by the open call stack between enter and exit.

### Variables

| kind | fields |
|------|--------|
| `VarCreate` | `name`, `value`, `span` |
| `VarDestroy` | `name`, `value`, `span` |
| `VarAssign` | `name`, `old?`, `value`, `span` |
| `Write` | `slot`, `old?`, `value`, `span` |
| `Swap` | `a`, `b`, `value_a`, `value_b`, `span` |
| `RefBind` | `name`, `target` (slot), `span` |
| `PtrMove` | `name`, `to`, `span` |
| `BuiltinSelect` | `name`, `args[]`, `chosen`, `value`, `span` | chooser builtins (`min` / `max`), span points to selected argument |

### Heap & containers

#### `Alloc` (critical for visualization)

```json
{
  "kind": "Alloc",
  "id": 3,
  "type_name": "vector",
  "size": 3,
  "elems": [
    { "kind": "Object", "value": 0 },
    { "kind": "Object", "value": 1 },
    { "kind": "Object", "value": 2 }
  ],
  "span": { "start": 43, "end": 76 }
}
```

| field | meaning |
|-------|---------|
| `id` | New heap object id |
| `type_name` | `"vector"`, `"string"`, `"map"`, class name, … |
| **`size`** | Logical length at allocation (vector len, string chars, map entry count, …) |
| **`elems`** | Snapshot of elements **at allocation** for sequences |
| **`entries`** | Map/set snapshot: `[{ "key": MapKey, "value": Value? }, …]` (`value` omitted for sets) |
| `span` | Source range of the allocating expression / decl |

**`elems` rules**

- **`vector` / `stack` / `queue`:** list of `ValueJson` (length === `size`). Nested vectors appear as `{ "kind": "Object", "value": <child_id> }` — those children have their **own** earlier `Alloc`s.
- **`pair`:** length 2: `[first, second]`.
- **`string`:** list of `{ "kind": "Char", "value": "…" }` (length === `size`).
- **`map` / `set` / unordered:** use **`entries`**, not `elems` (elems is `[]`).
- **`priority_queue`:** ints in heap order snapshot.
- Empty `vector` from `vector<int> v;`: `size: 0`, `elems: []`.

**How to draw `adj = {{0,0,0},{0,0,0},{0,0,0}}`**

1. Three `Alloc`s for row vectors (`size: 3`, int elems).
2. One `Alloc` for outer (`size: 3`, elems = Object ids of those rows).
3. `VarCreate` `adj` → `Object` with outer id.
4. Later `adj[i][j] = …` → `Write` / `ContainerMod` — update your shadow heap.

#### `map` / `unordered_map` `operator[]`

Missing key **default-inserts** a value of the mapped type (`map<K,V>` → default `V`), then returns it:

- `map<int,int>` → insert `0`
- `map<int, vector<int>>` → `Alloc` empty vector, insert that object id, then `m[0].push_back(...)` works

Events on first `m[k]` miss: `Write` (MapEntry) + `ContainerMod` with `op` / `kind` `"map_default_insert"`, plus any nested `Alloc` for the default value.

Range-for over a map with a **single** loop variable (`for (const auto& kvp : m)`) allocates a temporary `pair` each iteration: you get **`Alloc` (`kind: "pair"`)** then **`VarCreate`** pointing at that object id. Structured binding (`for (auto& [k, v] : m)`) binds key/value as scalars — no pair `Alloc`.

#### Other container events

| kind | fields | notes |
|------|--------|--------|
| `ContainerMod` | `container`, **`op`**, `index?`, **`key?`**, `old?`, `value?`, **`elems?`**, `span` | mutations; **`key` set for map/set**; **`elems` = after-state for bulk ops only** |

Bulk ops (`sort`, `reverse`, `iota`, `partial_sum`) set **`elems`** to the full after-state sequence (same encoding as `Alloc.elems`: scalars inline, nested containers as `{ "kind": "Object", "value": id }`). Incremental ops omit `elems` (empty / skipped in JSON).
| `ContainerLookup` | `container`, **`op`**, `key?`, `result`, `span` | reads: `count`, `size`, `empty`, `top`, `index`, … |
| `Dealloc` | `id`, `span` | object freed |

Pure assignment `nums[i] = x` does **not** emit a LHS `ContainerLookup` (only `Write` / `ContainerMod`). Compound assigns (`+=`) still look up the old value.

`ContainerLookup` with `op: "index"` is also emitted for subscript **reads** (`nums[i]`, `s[0]`, `m[k]` after resolve):

```json
{
  "kind": "ContainerLookup",
  "container": { "kind": "Object", "value": 0 },
  "op": "index",
  "key": { "kind": "Int", "value": 0 },
  "result": { "kind": "Int", "value": 2 },
  "span": { "start": 0, "end": 0 }
}
```

Note: pure assignment `nums[i] = x` does **not** emit a pre-store `ContainerLookup`; you only see `Write` / `ContainerMod`. Reads like `x = nums[i]` still emit `ContainerLookup` with `op: "index"`.

#### Common `ContainerMod` / `ContainerLookup` `op` strings

| op | container | notes |
|----|-----------|--------|
| `push_back` / `emplace_back` | vector | append |
| `stack::push` / `stack::emplace` | stack | same semantics; `emplace` is an alias of `push` |
| `stack::pop` | stack | |
| `set::insert` / `set::emplace` | set / unordered_set | same semantics |
| `sort` / `reverse` / `iota` / `partial_sum` | vector (via begin/end) | bulk rewrite; includes **`elems`** (after state, same shape as `Alloc.elems`) |
| `numeric_limits::min` / `max` / `lowest` | — | treated as `int` limits (`INT_MIN`/`INT_MAX`) |

Demos: `examples/two_sum.cpp` (`Solution::twoSum`), `examples/valid_parentheses.cpp` (`Solution::isValid` — stack events), `examples/dfs.cpp` (`Solution::countComponents`), `examples/n_queens.cpp` (`Solution::solveNQueens`). Local `testing/` smoke: `cargo test -p rscpp-runtime --test corpus_run`.

---

## Shadow-heap recipe (recommended UI model)

```text
heap: Map<id, { type_name, elems?: ValueJson[], entries?: {key, value?}[] }>

on Alloc:
  if kind is vector/string/stack/queue: heap[id] = { type_name, elems: copy(elems) }
  if kind is map/set (unordered_*):     heap[id] = { type_name, entries: copy(entries) }
  // size is always present; elems is [] for maps/sets; entries is [] for sequences

on Write Index:     skip — prefer paired ContainerMod (index_assign)
on Write MapEntry:  skip — prefer paired ContainerMod (map_default_insert / map_assign)
on ContainerMod:    if elems present (bulk op): replace heap[id].elems
                    else apply op (push_back → append; map_assign → upsert; …)
on Dealloc:         delete heap[id]
on VarCreate/Assign / Write Local: env[name] = value   // if Object, name → id
```

`VarAssign` and `Write { slot: Local }` are both emitted for the same local store — apply **one** (prefer `Write`) so you don't double-update. UI: silence `VarAssign`.

`Write { slot: MapEntry|Index }` and `ContainerMod` are both emitted for the same container store — apply **one** (prefer `ContainerMod`). UI: silence that `Write`. Both map keys use `MapKey` / `ValueJson` shape.

You do **not** need a separate `inspect(id)` API if you apply events in order.

---

## Intentional gaps (do not invent)

- No deep graph inside `Object` values — always an id.
- No stdin/stdout events in v1 — pass args via `run_method(source, "Solution::foo", argsJson)`.
- `Value::Str` exists for event map keys (not a heap string object).
- Step budget: runtime stops with `step limit exceeded` after ~100k steps (partial `events` still returned).

When this file and the Rust serializers disagree, **fix the serializers and update this doc in the same change.**
