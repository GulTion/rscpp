# LeetCode-subset STL containers & algorithms

**Date:** 2026-07-17  
**Status:** approved (fidelity **A** — LeetCode subset)  
**Goal:** Close the highest-value gaps in `SUPPORTED.md` for containers and `<algorithm>` so common LeetCode solutions load and run, without full ISO C++ fidelity.

## Fidelity (A)

- Prefer stubs and shared helpers over real iterators / allocators / complexity guarantees.
- Iterator pairs stay **begin/end stubs** resolved to a container id (existing `resolve_vector_range` model), extended to new sequence containers and reverse stubs where needed.
- Bounds / search algos keep returning **index `Int`** or **`Ptr` Index** as today — not real iterators.
- Skip: `vector<bool>` bit-packing, PBDS, `list::splice`/`merge`/`sort`, shuffle, `stable_sort`, projections / `ranges::*` beyond existing aliases, function-pointer comparators.

## Architecture

**Thin distinct heap objects + shared STL helpers** (not “everything is `Vector`”).

| C++ type | `Object` variant | Storage |
|----------|------------------|---------|
| `vector` | `Vector(Vec<Value>)` | existing |
| `deque` | `Deque(VecDeque<Value>)` | new |
| `list` | `List(Vec<Value>)` | new (vector-backed; no node semantics) |
| `array` | `Array { elems: Vec<Value>, n: usize }` | new; fixed length |
| `priority_queue` | `PriorityQueue` | evolve beyond `BinaryHeap<i64>` |

Map/set stay `BTreeMap` / `BTreeSet` / hash variants. Ordered bounds only on **ordered** map/set.

Sema: seed type names + soft member stubs as needed so parse/sema don’t block runtime.

Events: reuse `ContainerMod` / `ContainerLookup` with existing `kind` strings (`push_back`, `insert`, …).

## Phase map

| Phase | Scope |
|-------|--------|
| **P1** | `vector`: `at`, `reserve`/`capacity` (no-op or soft), `resize`, `insert`/`emplace`, `erase`, reverse iter stubs + reverse range construct |
| **P2** | `deque` full LeetCode surface |
| **P3** | `list` + `array` LeetCode surface |
| **P4** | `priority_queue` custom compare (`greater<>`, lambda) |
| **P5** | `map`/`set`: `rbegin`/`rend` stubs, `lower_bound`/`upper_bound`/`equal_range` |
| **P6** | `<algorithm>` non-modifying (subset) |
| **P7** | `<algorithm>` modifying (subset) |
| **P8** | `equal_range` (free) + merge / set ops on sorted sequences |

Each phase updates `SUPPORTED.md` and adds focused runtime tests under `crates/runtime/tests/`.

---

## P1 — `vector` gaps

| Member | Behavior |
|--------|----------|
| `at(i)` | Same as `operator[]`; optional bounds check → runtime error if OOB |
| `reserve(n)` | No-op (or store `capacity` soft field ignored by growth) |
| `capacity()` | Return `max(size, reserved)` or `size` if no-op reserve |
| `resize(n)` | Grow with default `Int(0)` / type default; shrink truncate |
| `resize(n, v)` | Grow filled with `v` |
| `insert(pos, v)` / `emplace(pos, …)` | `pos` = index int **or** `begin` stub meaning index 0 / `begin+i` if we already eval member begin as 0 — **LeetCode pattern:** `v.insert(v.begin()+i, x)` requires `begin()+i` to yield a usable position. Minimal: accept `Int` index; if arg is `Ptr` Index use its index; if `begin()`/`end()` stubs, treat as `0` / `size`. |
| `erase(pos)` / `erase(first, last)` | Same position model; return void or stub |
| `rbegin`/`rend`/`crbegin`/`crend` | Stubs; `resolve_*_range` detects reverse pair and iterates reversed (or builds reversed copy once for range-ctor / algos) |

**Construct `vector(v.rbegin(), v.rend())`:** when ctor/range-init sees reverse begin/end of same container, initialize with reversed elements.

---

## P2 — `deque`

Runtime object `Object::Deque(VecDeque<Value>)`.

Members (yes): ctor empty / brace; `size`/`empty`/`clear`; `[]`/`at`; `front`/`back`; `push_back`/`push_front`/`emplace_*`; `pop_back`/`pop_front`; `begin`/`end`/`cbegin`/`cend` stubs; range-for.

Wire: `engine` construct + `stl/deque.rs` + `stl/mod.rs` dispatch; extend `resolve_vector_range` → rename mentally to `resolve_sequence_range` supporting `Deque`/`List`/`Array`/`Vector`/`String`.

---

## P3 — `list` + `array`

**`list`:** `Object::List(Vec<Value>)`. Members like vector for LeetCode: `push_front`/`back`, `pop_front`/`back`, `front`/`back`, `size`/`empty`/`clear`, `insert`/`erase` by index stub, begin/end stubs. **No** `splice` / member `merge` / member `sort`. Allow `[]` for A if cheap (non-standard but fine for interpreter).

**`array`:** `Object::Array { elems, n }`. `size` always `n`; `[]`/`at`/`front`/`back`/`fill`; begin/end stubs. Construction: `array<int,N>` → `N` from type args when present; else length of brace init. No dynamic resize.

---

## P4 — `priority_queue` custom compare

Today: `BinaryHeap<i64>` max-heap.

Target:

```text
Object::PriorityQueue {
  elems: Vec<Value>,   // or keep heap of i64 when default
  cmp: Option<Value>,  // Closure | Functor object id wrapped as Value::Object
}
```

- Default cmp = max-heap on ints (`a < b` means a has lower priority).
- `priority_queue<int, vector<int>, greater<int>>` → template args mostly erased; detect `greater` in type name **or** ctor `priority_queue<…>(greater<>())` / assignment of cmp — **LeetCode A:** support (1) default max ints, (2) `greater<>` functor as third template name if preserved, (3) runtime `push` with stored Functor/Closure via binary heapify using existing `cmp_less`.
- Non-int `Value` optional later; A ships **ints (+ bool as 0/1)** first.

`top`/`push`/`pop`/`size`/`empty` unchanged in surface.

---

## P5 — `map` / `set` bounds & reverse stubs

**Ordered only** (`map` / `set`):

| Method | Return (A) |
|--------|------------|
| `lower_bound(k)` | index `Int` into sorted key list, or stub “iterator” as index (consistent with free `lower_bound`) |
| `upper_bound(k)` | same |
| `equal_range(k)` | `pair<Int,Int>` (lo, hi) via existing pair object |
| `rbegin`/`rend`/`cr*` | stubs; optional reverse key walk if something needs it |

`unordered_*`: bounds stay **no**.

---

## P6 — Non-modifying algorithms (subset)

All on resolved sequence containers; optional predicate = Closure/Functor.

| Algo | Notes |
|------|--------|
| `all_of` / `any_of` / `none_of` | unary pred |
| `find` | value `==` |
| `find_if` / `find_if_not` | pred; return index `Int` or `end` ≡ `size` |
| `count` / `count_if` | |
| `equal` | two ranges; optional pred |
| `search` / `search_n` | |
| `adjacent_find` | optional pred |

**Skip for A:** `find_end`, `find_first_of`, `mismatch`, `is_permutation`, `for_each` (unless trivial).

---

## P7 — Modifying algorithms (subset)

| Algo | Notes |
|------|--------|
| `fill` / `fill_n` | |
| `copy` / `copy_n` / `copy_if` | dest = begin of mutable sequence |
| `transform` | unary; binary form if easy |
| `replace` / `replace_if` | |
| `remove` / `remove_if` | partition + return new logical end index; caller `erase` |
| `unique` | consecutive; return new end index |
| `rotate` | |
| `generate` / `generate_n` | nullary callable |

**Skip for A:** `shuffle`, `random_shuffle`, `copy_backward`, `swap_ranges`, `iter_swap`, `unique_copy` / `remove_copy*` / `reverse_copy` / `rotate_copy` unless one-liners.

---

## P8 — `equal_range` + merge / set ops

| Algo | Notes |
|------|--------|
| `equal_range(b,e,x[,cmp])` | return `pair` of indexes `(lo, hi)` |
| `merge` | two sorted in → dest begin |
| `includes` | bool |
| `set_union` / `set_intersection` / `set_difference` / `set_symmetric_difference` | sorted int sequences → dest |

`inplace_merge`: optional if cheap; else **no** for A.

Cmp overloads: same as existing bounds (`cmp_less`).

---

## Testing & docs

- Per-phase integration tests in `crates/runtime/tests/` (small C++ snippets + `Engine::from_source` / `run_main` or `call`).
- Update `SUPPORTED.md` rows from `no` → `yes` / `partial` with notes matching A.
- Regression: existing `run.rs` / `functional_ops.rs` / algo comparator tests stay green.

## Success criteria

1. Listed members/algos marked appropriately in `SUPPORTED.md`.
2. Representative LeetCode-shaped snippets execute (synthetic args OK).
3. No claim of full ISO; notes document stub iterator / index returns.
