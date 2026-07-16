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
  error?: string;        // present when ok === false
};
```

No stdin/stdout in v1 — reconstruct program state from **events** (and `value`).

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
| `MapEntry` | `{ "obj": number, "key": string }` |

---

## Slots (`SlotJson`)

Where a write happened (for highlighting):

| `kind` | fields |
|--------|--------|
| `Local` | `name` |
| `Global` | `name` |
| `Object` | `obj` (heap id) |
| `Index` | `obj`, `index` |
| `MapEntry` | `obj`, `key` |
| `Field` | `obj`, `field` |

---

## Events (`EventJson`)

Every event has `"kind": "<Name>"` plus fields. Common: `span`.

### Stepping / control

| kind | fields | notes |
|------|--------|--------|
| `Step` | `call_id?`, `span` | about to run a statement |
| `ScopeEnter` / `ScopeExit` | `call_id?`, `span` | `{` / `}` |
| `Branch` | `call_id?`, `then_taken`, `span` | `if` path |
| `LoopIter` | `call_id?`, `span` | start of a loop body iteration |
| `Compare` | `call_id?`, `op`, `left`, `right`, `result`, `span` | |
| `FnEnter` | `name`, **`call_id`**, **`parent_id?`**, `args[]`, `span` | activation edge for call trees |
| `FnExit` | `name`, **`call_id`**, **`parent_id?`**, `ret`, `span` | same ids as matching enter |

`call_id` on non-Fn events is the **current** activation (which frame is running). Use it with `FnEnter`/`FnExit` to attribute heap/var ops to a recursion frame.

### Variables

| kind | fields |
|------|--------|
| `VarCreate` | `call_id?`, `name`, `value`, `span` |
| `VarAssign` | `call_id?`, `name`, `old?`, `value`, `span` |
| `Write` | `call_id?`, `slot`, `old?`, `value`, `span` |
| `Swap` | `call_id?`, `a`, `b`, `value_a`, `value_b`, `span` |
| `RefBind` | `call_id?`, `name`, `target` (slot), `span` |
| `PtrMove` | `call_id?`, `name`, `to`, `span` |

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
| `call_id?` | Current activation |
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

#### Other container events

| kind | fields | notes |
|------|--------|--------|
| `ContainerMod` | `call_id?`, `container`, **`op`**, `index?`, **`key?`**, `old?`, `value?`, `span` | mutations; **`key` set for map/set** |
| `ContainerLookup` | `call_id?`, `container`, **`op`**, `key?`, `result`, `span` | reads: `count`, `size`, `empty`, `top`, `index`, … |
| `Dealloc` | `call_id?`, `id`, `span` | object freed |

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

---

## Shadow-heap recipe (recommended UI model)

```text
heap: Map<id, { type_name, elems: ValueJson[] }>

on Alloc:     heap[id] = { type_name, elems: copy(elems) }   // size = elems.length for vectors
on Write Index: heap[obj].elems[index] = value
on ContainerMod push_back: append value; etc.
on Dealloc:   delete heap[id]
on VarCreate/Assign: env[name] = value   // if Object, name → id
```

You do **not** need a separate `inspect(id)` API if you apply events in order.

---

## Intentional gaps (do not invent)

- No deep graph inside `Object` values — always an id.
- No stdin/stdout events in v1.
- `Value::Str` exists for event map keys (not a heap string object).

When this file and the Rust serializers disagree, **fix the serializers and update this doc in the same change.**
