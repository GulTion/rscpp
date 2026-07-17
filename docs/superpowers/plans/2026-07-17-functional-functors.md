# `<functional>` arithmetic / comparison / logical / bitwise functors

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make standard function objects from `<functional>` (arithmetic, comparisons, logical, bitwise) work at runtime so LeetCode patterns like `sort(..., greater<int>())` and `accumulate(..., plus<int>())` execute, then mark them **yes** in `SUPPORTED.md`.

**Architecture:** Parser already erases `<…>` on expression names, so `greater<int>()` is `Call { Name("greater"), args: [] }`. Allocate a heap `Object::Functor { kind }` on zero-arg construction; on call (`g(a,b)` or as algo cmp), apply the op. Reuse the same call path as closures for algo `cmp` / `accumulate` op.

**Tech Stack:** Rust `rscpp-runtime` (`value::Object`, `engine/expr.rs` call path, `engine/algo.rs` `cmp_less` / accumulate); update `SUPPORTED.md`.

## Global Constraints

- **Runtime-first** — status in `SUPPORTED.md` means tree-walker runtime, not sema stubs alone.
- **Int (and Bool where natural) only** — `as_int` / `as_bool`; no float/string functors in v1.
- **Template args ignored** — `greater<int>()`, `greater<>()`, `greater()` all identical (parser erases `<…>`).
- **No** `ranges::` constrained functors, transparent specialization quirks, or `void`/`T` specialization differences.
- **No** new Event kinds — optional: reuse `ContainerLookup`-style only if already needed; prefer silent ops.
- Lambdas as cmp remain supported; functors are an alternate callable shape.
- Update `SUPPORTED.md` in the **same** change that lands runtime support.
- Commits only when the user asks.

---

## Design (locked)

### Construction

| Source expression | Runtime |
|-------------------|---------|
| `greater<int>()` / `greater<>()` / `std::greater<int>()` | Eval zero-arg Call of name `greater` → `Value::Object(id)` of `Object::Functor { kind: Greater }` |
| Same for all listed functor names | Same pattern |

Strip `std::` prefix when matching names (same as algo builtins).

### Invocation

| Call shape | Behavior |
|------------|----------|
| `g(a, b)` where `g` is Functor (binary) | Apply op; return `Int` or `Bool` |
| `g(a)` where `g` is unary (`negate`, `logical_not`, `bit_not`) | Apply op |
| Passed as 3rd arg to `sort` / bounds / `binary_search` / min/max_element | `cmp_less` must accept Functor like Closure: call with `(a,b)` expect Bool |
| Passed as 4th arg to `accumulate` | Call with `(acc, elem)` expect Int (for `plus` etc.) |

Wrong arity → `RuntimeError` with clear message.

### Functor kinds (v1 set)

**Arithmetic (binary except negate):** `plus`, `minus`, `multiplies`, `divides`, `modulus`, `negate`  
**Comparisons (binary → Bool):** `equal_to`, `not_equal_to`, `greater`, `less`, `greater_equal`, `less_equal`  
**Logical:** `logical_and`, `logical_or` (binary Bool), `logical_not` (unary Bool)  
**Bitwise:** `bit_and`, `bit_or`, `bit_xor` (binary Int), `bit_not` (unary Int)

Division / modulo by zero → same error style as binary `/` `%` in the engine.

### Out of scope (this plan)

- `hash`, `bind`, `ref`/`cref`, `function` improvements, searchers, `not_fn`, `identity`
- `ranges::*` comparisons / `compare_three_way`
- `priority_queue` custom Compare template
- VM parity (tree-walker only unless trivial)

---

## File structure

| Path | Responsibility |
|------|----------------|
| `crates/runtime/src/value.rs` | Add `Object::Functor { kind: FunctorKind }` + `FunctorKind` enum; `type_name` / `alloc_snapshot` / `len` / `clear` arms |
| `crates/runtime/src/stl/functor.rs` (new) **or** `engine/functor.rs` | `kind_from_name`, `arity`, `apply(kind, args) -> Result<Value>` |
| `crates/runtime/src/engine/expr.rs` | Zero-arg Call → construct Functor; Call when callee is Functor → `apply` |
| `crates/runtime/src/engine/algo.rs` | `cmp_less` / accumulate op: if Functor, `apply` like Closure |
| `crates/runtime/src/engine/call.rs` | If needed: shared `call_callable(id, args)` used by closure + functor (optional DRY) |
| `crates/runtime/tests/stl.rs` or `run.rs` | Construction + `g(a,b)` + `sort(..., greater<int>())` + `accumulate(..., plus<int>())` |
| `SUPPORTED.md` | Mark the three sections **yes** with notes |

Prefer **one** `engine/functor.rs` (or `stl/functor.rs`) with pure `apply` to keep `expr.rs` thin.

---

### Task 1: `Object::Functor` + apply table

**Files:**
- Create: `crates/runtime/src/engine/functor.rs`
- Modify: `crates/runtime/src/value.rs`
- Modify: `crates/runtime/src/lib.rs` or `engine/mod.rs` (`mod functor`)
- Test: `crates/runtime/tests/functional_ops.rs` (new)

**Interfaces:**
- Produces:
  - `pub enum FunctorKind { Plus, Minus, … }`
  - `pub fn functor_kind(name: &str) -> Option<FunctorKind>` (strips `std::`)
  - `pub fn functor_apply(kind: FunctorKind, args: &[Value], span: Span) -> Result<Value, RuntimeError>`
- Consumes: `Value::as_int` / `as_bool`

- [ ] **Step 1: Write failing tests**

```rust
// crates/runtime/tests/functional_ops.rs
use rscpp_runtime::{Engine, Value};

#[test]
fn greater_construct_and_call() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  auto g = greater<int>();
  return g(3, 1) ? 1 : 0;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(1));
}

#[test]
fn plus_binary() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  auto p = plus<int>();
  return p(2, 5);
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(7));
}
```

- [ ] **Step 2: Run tests — expect FAIL** (`undefined function` or similar)

```bash
cargo test -p rscpp-runtime --test functional_ops -- --nocapture
```

- [ ] **Step 3: Add `FunctorKind` + `Object::Functor` in `value.rs`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctorKind {
    Plus, Minus, Multiplies, Divides, Modulus, Negate,
    EqualTo, NotEqualTo, Greater, Less, GreaterEqual, LessEqual,
    LogicalAnd, LogicalOr, LogicalNot,
    BitAnd, BitOr, BitXor, BitNot,
}

// in Object:
Functor { kind: FunctorKind },
```

Update `type_name` → `"functor"`, `len`/`clear`/`alloc_snapshot` no-ops like Closure.

- [ ] **Step 4: Implement `functor.rs` apply table**

Match kind + args.len(); binary compare → `Value::Bool`; arithmetic/bitwise → `Value::Int`; logical use `as_bool`.

- [ ] **Step 5: Wire construction + call in `expr.rs`**

In free-name Call handling (before “undefined function”):

1. If `args.is_empty()` and `functor_kind(&name).is_some()` → alloc `Object::Functor`, `emit_alloc` optional (skip Alloc noise for functors — **no Alloc event** unless easy; prefer none).
2. After evaluating callee to `Value::Object(id)`, if `Functor`, `functor_apply` and return.

Also handle `std::greater` name form.

- [ ] **Step 6: Re-run `functional_ops` — expect PASS**

- [ ] **Step 7: Commit if user requested**

---

### Task 2: Algo integration (`sort` / bounds / `accumulate`)

**Files:**
- Modify: `crates/runtime/src/engine/algo.rs` (`cmp_less`, `builtin_accumulate`)
- Modify: `crates/runtime/tests/functional_ops.rs`
- Test: same

**Interfaces:**
- Consumes: `Object::Functor`, `functor_apply`
- Produces: functors usable wherever Closure cmp/op is today

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn sort_with_greater() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(3); v.push_back(2);
  sort(v.begin(), v.end(), greater<int>());
  return v[0] * 100 + v[1] * 10 + v[2];
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(321));
}

#[test]
fn accumulate_with_plus() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(2); v.push_back(3);
  return accumulate(v.begin(), v.end(), 0, plus<int>());
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(6));
}
```

- [ ] **Step 2: Run — expect FAIL** (sort cmp must be lambda)

- [ ] **Step 3: Extend `cmp_less`**

```rust
// After Closure branch:
if let Some(Object::Functor { kind }) = self.heap.get(*opid).cloned() {
    let v = functor_apply(kind, &[a.clone(), b.clone()], span)?;
    return v.as_bool().map_err(RuntimeError::new);
}
```

(Use clone of kind to avoid borrow issues.)

- [ ] **Step 4: Extend `builtin_accumulate` op branch** similarly with `functor_apply`.

- [ ] **Step 5: Allow sort’s comparator check** to accept Functor (today errors “must be a lambda” — change to Closure **or** Functor).

- [ ] **Step 6: Tests PASS**

```bash
cargo test -p rscpp-runtime --test functional_ops
cargo test -p rscpp-runtime --test run algo_comparators_descending
```

- [ ] **Step 7: Commit if requested**

---

### Task 3: Cover remaining ops + edge cases

**Files:**
- Modify: `crates/runtime/tests/functional_ops.rs`
- Modify: `functor.rs` only if gaps found

- [ ] **Step 1: Add tests** for each kind at least once:

| Kind | Expression | Expected |
|------|------------|----------|
| `less` | `less<int>()(1,2)` | true |
| `multiplies` | `multiplies<int>()(3,4)` | 12 |
| `divides` / `modulus` | non-zero denom | ok |
| `divides` zero | `divides<int>()(1,0)` | error |
| `negate` | `negate<int>()(5)` | -5 |
| `logical_and` / `logical_or` / `logical_not` | bool-ish ints | Bool |
| `bit_and` / `bit_or` / `bit_xor` / `bit_not` | ints | Int |
| `equal_to` / `not_equal_to` / `greater_equal` / `less_equal` | ints | Bool |

- [ ] **Step 2: Implement any missing arms; PASS**

- [ ] **Step 3: Commit if requested**

---

### Task 4: Update `SUPPORTED.md`

**Files:**
- Modify: `SUPPORTED.md` (`## Function objects` Arithmetic / Comparisons / Logical / bitwise tables)

- [ ] **Step 1: Set status to `yes`** for:

  - Arithmetic: `plus` … `negate`
  - Comparisons: `equal_to` … `less_equal` (not `ranges::*`)
  - Logical / bitwise: all four logical + four bit ops

- [ ] **Step 2: Notes column:** `greater<int>()` etc.; template args erased; ints (bools for logical); usable as sort/accumulate callables.

- [ ] **Step 3: Keep `ranges::*` / hash / bind as **no**.

- [ ] **Step 4: Commit if requested**

---

### Task 5: Regression gate

- [ ] **Step 1:**

```bash
cargo test -p rscpp-runtime --tests 2>&1 | tail -40
```

Expected: all PASS (ignored probe OK).

- [ ] **Step 2:** Spot-check `SUPPORTED.md` rows match reality (greater construct + sort).

---

## Self-review

1. **Coverage:** Arithmetic, Comparisons, Logical, Bitwise from user ask → Tasks 1–4.
2. **Parser constraint:** Documented erasure of `<T>` — no AST change required.
3. **Algo path:** Explicit Task 2 so `sort(..., greater<int>())` does not stay Closure-only.

---

## Execution handoff

Plan saved to `docs/superpowers/plans/2026-07-17-functional-functors.md`.

**1. Subagent-Driven** — fresh subagent per task + review  
**2. Inline Execution** — this session with checkpoints  

Which approach?
