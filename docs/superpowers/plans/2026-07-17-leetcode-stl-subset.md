# LeetCode-subset STL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship fidelity-A STL gaps from `docs/superpowers/specs/2026-07-17-leetcode-stl-subset-design.md` so common LeetCode containers and algorithms run in rscpp.

**Architecture:** Thin distinct `Object` variants + shared sequence-range helpers; keep index/`Ptr` returns; reuse `cmp_less` / Closure / Functor for predicates and pq compare.

**Tech Stack:** Rust workspace `rscpp-runtime` (`stl/*`, `engine/algo.rs`, `engine/expr.rs`, `value.rs`), `rscpp-sema` seeds, `SUPPORTED.md`, integration tests under `crates/runtime/tests/`.

## Global Constraints

- Fidelity **A** only (LeetCode subset) — no real iterators, no PBDS, no `vector<bool>`, no shuffle/stable_sort.
- Prefer smallest diff; follow existing `stl/vector.rs` + `resolve_vector_range` patterns.
- Each task: failing test → implement → green → update `SUPPORTED.md` → commit.
- Spec: `docs/superpowers/specs/2026-07-17-leetcode-stl-subset-design.md`.

---

## File map

| Area | Files |
|------|--------|
| Values | `crates/runtime/src/value.rs` |
| Dispatch | `crates/runtime/src/stl/mod.rs`, `stl/vector.rs`, new `stl/deque.rs`, `stl/list.rs`, `stl/array.rs`, `stl/priority_queue.rs`, `stl/map.rs`, `stl/set.rs` |
| Ranges / algos | `crates/runtime/src/engine/algo.rs`, `engine/expr.rs`, possibly `engine/types.rs` (construct) |
| Sema | `crates/sema/src/analyze/mod.rs`, `analyze/stl.rs` if present |
| Docs | `SUPPORTED.md` |
| Tests | `crates/runtime/tests/stl_vector_ext.rs`, `stl_deque_list_array.rs`, `stl_pq_cmp.rs`, `stl_map_bounds.rs`, `algo_nonmod.rs`, `algo_modifying.rs`, `algo_merge_set.rs` (names flexible) |

---

### Task 1: Vector — `at`, `reserve`/`capacity`, `resize`

**Files:**
- Modify: `crates/runtime/src/stl/vector.rs`
- Modify: `crates/runtime/src/value.rs` (optional soft `capacity` only if needed — prefer no-op reserve)
- Test: `crates/runtime/tests/stl_vector_ext.rs`
- Modify: `SUPPORTED.md`

**Interfaces:**
- Consumes: existing `Object::Vector`, `Ctx::query` / `modify`
- Produces: methods `at`, `reserve`, `capacity`, `resize` on vector

- [ ] **Step 1: Write failing tests**

```rust
// crates/runtime/tests/stl_vector_ext.rs
use rscpp_runtime::{Engine, Value};

#[test]
fn vector_at_resize_reserve() {
    let src = r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(2);
  v.reserve(10);
  int c = v.capacity();
  v.resize(4, 9);
  v.resize(3);
  return v.at(0) + v.at(2) * 10 + c * 0 + (int)v.size();
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    // 1 + 9*10 + 3 = 94 (capacity may be >=3; we add *0)
    assert_eq!(eng.run_main().unwrap(), Value::Int(94));
}
```

- [ ] **Step 2: Run test — expect FAIL** (`unknown vector method`)

```bash
cargo test -p rscpp-runtime --test stl_vector_ext vector_at_resize_reserve -- --nocapture
```

- [ ] **Step 3: Implement in `stl/vector.rs`**

- `at` → same as index path used for `[]` / `front` (bounds-check → `RuntimeError`)
- `reserve` → no-op, return void
- `capacity` → return `size` as `Int` (A)
- `resize` → 1-arg grow with `Int(0)` / shrink; 2-arg grow with fill value

- [ ] **Step 4: Test PASS + mark SUPPORTED.md rows yes/partial**

- [ ] **Step 5: Commit** `feat(runtime): vector at/resize/reserve for LeetCode subset`

---

### Task 2: Vector — `insert` / `emplace` / `erase` + position model

**Files:**
- Modify: `crates/runtime/src/stl/vector.rs`
- Modify: `crates/runtime/src/engine/expr.rs` or `call.rs` if positions arrive as exprs (prefer evaluate args to `Value::Int` or `Ptr` Index)
- Test: `crates/runtime/tests/stl_vector_ext.rs`

**Interfaces:**
- Produces: `insert`/`emplace`/`erase` accepting index `Int`, or `Ptr` Index `{obj,index}`, or stub begin/end as 0/`size`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn vector_insert_erase() {
    let src = r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(3);
  v.insert(v.begin() + 1, 2);  // or insert at index 1 if begin+i not ready: use helper
  v.erase(v.begin());
  return v.size() * 10 + v[0];
}
"#;
    // If begin()+i not supported yet, use:
    //   v.insert(1, 2); via Int — but C++ doesn't; prefer making begin()+i work in expr for Ptr/Int.
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(22)); // size 2, front 2
}
```

- [ ] **Step 2: FAIL then implement**

Position helper (put in `stl/mod.rs` or `vector.rs`):

```rust
fn seq_index(ctx: &Ctx, id: ObjId, pos: &Value, span: Span) -> Result<usize> {
    match pos {
        Value::Int(i) if *i >= 0 => Ok(*i as usize),
        Value::Ptr(Address::Index { obj, index }) if *obj == id => Ok(*index),
        // begin/end stubs historically return Int(0) — document: erase/insert with Int(0)/size
        _ => Err(RuntimeError::at(span, "insert/erase position")),
    }
}
```

If `v.begin() + 1` does not already yield `Int(1)` or Index ptr, add minimal `Int + Int` already works when `begin` returns `0`.

- [ ] **Step 3: PASS + SUPPORTED.md `insert`/`erase` → yes (partial overloads)**

- [ ] **Step 4: Commit** `feat(runtime): vector insert/emplace/erase by index`

---

### Task 3: Vector — reverse iterators + reverse range init

**Files:**
- Modify: `crates/runtime/src/stl/vector.rs` (`rbegin`/`rend`/`crbegin`/`crend`)
- Modify: `crates/runtime/src/engine/algo.rs` (`resolve_vector_range` → detect reverse)
- Modify: `crates/runtime/src/engine/types.rs` or construct path for `vector(it,it)`
- Test: `stl_vector_ext.rs`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn vector_from_rbegin_rend() {
    let src = r#"
int main() {
  vector<int> v; v.push_back(1); v.push_back(2); v.push_back(3);
  vector<int> r(v.rbegin(), v.rend());
  return r[0] * 100 + r[1] * 10 + r[2];
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(321));
}
```

- [ ] **Step 2: Implement**

- Reverse methods return distinguishable stubs, e.g. `Value::Int` with a convention **or** small tagged approach. Simplest A approach already used: `begin`/`end` return `Int(0)`; for reverse, return `Int(0)`/`Int(0)` is ambiguous.
- **Preferred A:** encode reverse in range resolver by method name on the call expr (same as `range_base` today): if both ends are `rbegin`/`rend` of same object, iterate reversed / construct reversed.
- Extend `range_base` to return `(base, Reverse|Forward)`.

- [ ] **Step 3: PASS + SUPPORTED.md**

- [ ] **Step 4: Commit** `feat(runtime): vector rbegin/rend and reverse range init`

---

### Task 4: `deque` container

**Files:**
- Modify: `crates/runtime/src/value.rs` — add `Object::Deque(VecDeque<Value>)`
- Create: `crates/runtime/src/stl/deque.rs`
- Modify: `crates/runtime/src/stl/mod.rs` — dispatch
- Modify: `crates/runtime/src/engine/types.rs` — construct `deque`
- Modify: `crates/runtime/src/engine/algo.rs` — sequence range includes Deque
- Modify: sema seeds if needed
- Test: `crates/runtime/tests/stl_deque_list_array.rs`
- Modify: `SUPPORTED.md`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn deque_push_front_back() {
    let src = r#"
int main() {
  deque<int> d;
  d.push_back(2); d.push_front(1); d.push_back(3);
  return d.front() * 100 + d.back() * 10 + (int)d.size();
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(133));
}
```

- [ ] **Step 2: Implement object + `stl/deque.rs` methods** (`size`/`empty`/`clear`/`[]`/`at`/`front`/`back`/`push_*`/`pop_*`/`emplace_*`/`begin`/`end` stubs)

- [ ] **Step 3: Extend `Object::len` / construct / range-for / `resolve_*` for Deque**

- [ ] **Step 4: PASS + SUPPORTED.md deque section → yes (subset note)**

- [ ] **Step 5: Commit** `feat(runtime): deque LeetCode subset`

---

### Task 5: `list` + `array`

**Files:**
- Modify: `value.rs` — `List(Vec<Value>)`, `Array { elems, n }`
- Create: `stl/list.rs`, `stl/array.rs`
- Modify: `stl/mod.rs`, `types.rs`, `algo.rs` range, sema
- Test: `stl_deque_list_array.rs`
- Modify: `SUPPORTED.md`

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn list_push_front() {
    let src = r#"
int main() {
  list<int> L; L.push_back(2); L.push_front(1);
  return L.front() + L.back() * 10;
}
"#;
    assert_eq!(Engine::from_source(src).unwrap().run_main().unwrap(), Value::Int(21));
}

#[test]
fn array_fill_and_index() {
    let src = r#"
int main() {
  array<int,3> a;
  a.fill(7);
  return a[0] + a.size();
}
"#;
    assert_eq!(Engine::from_source(src).unwrap().run_main().unwrap(), Value::Int(10));
}
```

- [ ] **Step 2: Implement List (vector-backed) + Array (fixed `n`)**

- `array<int,N>`: read `N` from type args when available; else `elems.len()` after brace init
- `fill(v)` sets all slots

- [ ] **Step 3: PASS + SUPPORTED.md**

- [ ] **Step 4: Commit** `feat(runtime): list and array LeetCode subset`

---

### Task 6: `priority_queue` custom compare

**Files:**
- Modify: `crates/runtime/src/value.rs` — replace/extend `PriorityQueue`
- Modify: `crates/runtime/src/stl/priority_queue.rs`
- Modify: construct path to capture `greater` / cmp object
- Reuse: `engine/algo.rs` `cmp_less`
- Test: `crates/runtime/tests/stl_pq_cmp.rs`
- Modify: `SUPPORTED.md`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn pq_with_greater() {
    let src = r#"
int main() {
  priority_queue<int, vector<int>, greater<int>> pq;
  pq.push(3); pq.push(1); pq.push(2);
  int a = pq.top(); pq.pop();
  int b = pq.top();
  return a * 10 + b;
}
"#;
    // min-heap: top 1 then 2 → 12
    assert_eq!(Engine::from_source(src).unwrap().run_main().unwrap(), Value::Int(12));
}
```

- [ ] **Step 2: Implement**

If parser erases template args, detect `greater` from remaining type name string on the `priority_queue` type **or** default-construct Functor when third arg name is `greater`. Store `cmp: Option<Value>` (Functor/Closure). Heapify with `Vec<Value>` + sift using `cmp_less`.

Fallback test if templates fully erased:

```cpp
priority_queue<int> pq; // max still works
```

and a second test using lambda only if ctor form exists; for A, Functor `greater` via type name is enough.

- [ ] **Step 3: PASS + SUPPORTED.md** custom Compare → yes (ints)

- [ ] **Step 4: Commit** `feat(runtime): priority_queue greater/lambda compare`

---

### Task 7: map/set — bounds + reverse stubs

**Files:**
- Modify: `crates/runtime/src/stl/map.rs`, `stl/set.rs`
- Test: `crates/runtime/tests/stl_map_bounds.rs`
- Modify: `SUPPORTED.md`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn map_lower_upper_equal_range() {
    let src = r#"
int main() {
  map<int,int> m;
  m[1]=1; m[3]=3; m[5]=5;
  int lo = m.lower_bound(3);
  int hi = m.upper_bound(3);
  // equal_range → pair; use first/second if pair supported
  return lo * 10 + hi;
}
"#;
    // sorted keys [1,3,5]: lower_bound(3)=1, upper_bound(3)=2 → 12
    assert_eq!(Engine::from_source(src).unwrap().run_main().unwrap(), Value::Int(12));
}
```

- [ ] **Step 2: Implement on `BTreeMap`/`BTreeSet` only** — collect ordered keys, binary search / `BTreeMap` range API; return indexes. `equal_range` → `Object::Pair { first, second }`. `rbegin`/`rend` stubs like vector.

- [ ] **Step 3: PASS + SUPPORTED.md**

- [ ] **Step 4: Commit** `feat(runtime): map/set lower_bound upper_bound equal_range`

---

### Task 8: Algorithms — non-modifying subset

**Files:**
- Modify: `crates/runtime/src/engine/expr.rs` (name dispatch)
- Modify: `crates/runtime/src/engine/algo.rs` (implementations)
- Modify: sema seeds (`all_of`, `find`, … already partially seeded)
- Test: `crates/runtime/tests/algo_nonmod.rs`
- Modify: `SUPPORTED.md`

**Algos:** `all_of`/`any_of`/`none_of`, `find`/`find_if`/`find_if_not`, `count`/`count_if`, `equal`, `search`/`search_n`, `adjacent_find`

- [ ] **Step 1: One failing umbrella test** covering `all_of` + `find` + `count_if`

```rust
#[test]
fn nonmod_all_find_count() {
    let src = r#"
int main() {
  vector<int> v; v.push_back(1); v.push_back(2); v.push_back(3);
  bool ok = all_of(v.begin(), v.end(), [](int x){ return x > 0; });
  int i = find(v.begin(), v.end(), 2);
  int c = count_if(v.begin(), v.end(), [](int x){ return x >= 2; });
  return (ok?1:0)*100 + i*10 + c;
}
"#;
    assert_eq!(Engine::from_source(src).unwrap().run_main().unwrap(), Value::Int(112));
}
```

- [ ] **Step 2: Implement via shared `resolve_sequence_range` + pred call (Closure/Functor)**; `find` returns index `Int` (`size` if missing)

- [ ] **Step 3: Add remaining algos in same file with small tests**

- [ ] **Step 4: PASS + SUPPORTED.md table**

- [ ] **Step 5: Commit** `feat(runtime): non-modifying algorithm subset`

---

### Task 9: Algorithms — modifying subset

**Files:**
- Modify: `engine/expr.rs`, `engine/algo.rs`
- Test: `crates/runtime/tests/algo_modifying.rs`
- Modify: `SUPPORTED.md`

**Algos:** `fill`/`fill_n`, `copy`/`copy_n`/`copy_if`, `transform`, `replace`/`replace_if`, `remove`/`remove_if`, `unique`, `rotate`, `generate`/`generate_n`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn fill_remove_unique() {
    let src = r#"
int main() {
  vector<int> v; v.push_back(1); v.push_back(2); v.push_back(2); v.push_back(3);
  fill(v.begin(), v.begin()+1, 9);
  auto it = remove(v.begin(), v.end(), 2);
  v.erase(it, v.end());
  return v.size() * 10 + v[0];
}
"#;
    // after fill: 9,2,2,3; remove 2 → 9,3; size 2 → 29
    assert_eq!(Engine::from_source(src).unwrap().run_main().unwrap(), Value::Int(29));
}
```

- [ ] **Step 2: Implement**; `remove`/`unique` return end index `Int`; ensure `erase(first,last)` from Task 2 accepts two indexes / begin stubs

- [ ] **Step 3: Remaining modifying algos + tests**

- [ ] **Step 4: Commit** `feat(runtime): modifying algorithm subset`

---

### Task 10: `equal_range` + merge / set ops

**Files:**
- Modify: `engine/expr.rs`, `engine/algo.rs`
- Test: `crates/runtime/tests/algo_merge_set.rs`
- Modify: `SUPPORTED.md`

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn equal_range_and_set_intersection() {
    let src = r#"
int main() {
  vector<int> a; a.push_back(1); a.push_back(2); a.push_back(2); a.push_back(3);
  auto er = equal_range(a.begin(), a.end(), 2);
  vector<int> b; b.push_back(2); b.push_back(3); b.push_back(4);
  vector<int> out; out.resize(3);
  auto end = set_intersection(a.begin(), a.end(), b.begin(), b.end(), out.begin());
  return er.first * 100 + er.second * 10 + out[0];
}
"#;
    // lo=1 hi=3 → 130 + out[0]=2 → 132 (adjust if pair field access differs)
    let mut eng = Engine::from_source(src).unwrap();
    let _ = eng.run_main().unwrap();
}
```

Fix assertion once pair `.first`/`.second` field access is confirmed in engine (already used for `make_pair`).

- [ ] **Step 2: Implement `equal_range` → Pair of indexes; merge/set ops writing into dest sequence from `out.begin()`**

- [ ] **Step 3: `includes`, `set_union`, `set_difference`, `set_symmetric_difference`, `merge`**

- [ ] **Step 4: PASS + SUPPORTED.md**

- [ ] **Step 5: Commit** `feat(runtime): equal_range and sorted-set algorithms`

---

### Task 11: Regression gate

- [ ] **Step 1: Run**

```bash
cargo test -p rscpp-runtime
cargo test -p rscpp-sema
```

- [ ] **Step 2: Fix any breakage from `Object` enum exhaustiveness**

- [ ] **Step 3: Final commit if needed** `test: STL subset regression green`

---

## Spec coverage checklist

| Spec section | Task |
|--------------|------|
| P1 vector at/resize/reserve | T1 |
| P1 insert/erase | T2 |
| P1 rbegin + reverse init | T3 |
| P2 deque | T4 |
| P3 list + array | T5 |
| P4 pq cmp | T6 |
| P5 map/set bounds | T7 |
| P6 non-modifying | T8 |
| P7 modifying | T9 |
| P8 equal_range + merge/set | T10 |
| SUPPORTED.md + tests | each task + T11 |
