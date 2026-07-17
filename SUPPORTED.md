# rscpp support matrix

Living checklist of what the interpreter actually runs. Tell the agent “support X from SUPPORTED.md” (or “mark X done”) to drive work.

Legend: **yes** · **partial** · **no**

---

## Language

| Feature | Status | Notes |
|---------|--------|--------|
| Classes / methods / `this` | yes | Nested helpers OK |
| Constructors + member-init lists | yes | e.g. `: set_(n)` |
| Lambdas / closures | yes | Captures `[&]`-style snapshot; call as locals |
| `using Alias = Type` | yes | File- and class-scope |
| `auto` / `const auto&` | partial | Common cases; not full C++ deduction |
| Templates | no | STL types are builtins, not real templates |
| Inheritance / virtual | no | |
| Exceptions / try-catch | no | |
| Preprocessor `#define` / macros | no | Includes ignored / stubbed |
| Pointer arithmetic beyond STL | partial | Heap/Index/`nullptr` compares |
| References | partial | Params / locals; not full binding rules |

---

## STL containers

Member checklists vs [cplusplus.com](https://cplusplus.com/reference/). Status = **runtime** (sema may accept more names than we execute).

### `vector`

Ref: [vector](https://cplusplus.com/reference/vector/vector/). Runtime: `stl/vector.rs` + `[]` via engine.

| Member | Status | Notes |
|--------|--------|--------|
| **(constructor)** | partial | Default / brace init; not full ctor overloads |
| **(destructor)** | yes | |
| **operator=** | partial | Normal assign; not full overload set |
| **begin** / **end** / **cbegin** / **cend** | partial | Stubs for algos / range-for |
| **rbegin** / **rend** / **crbegin** / **crend** | no | |
| **size** | yes | |
| **max_size** | no | |
| **resize** | no | |
| **capacity** | no | |
| **empty** | yes | |
| **reserve** | no | |
| **shrink_to_fit** | no | |
| **operator[]** | yes | |
| **at** | no | |
| **front** / **back** | yes | |
| **data** | no | |
| **assign** | no | |
| **push_back** / **emplace_back** | yes | |
| **pop_back** | yes | |
| **insert** / **emplace** | no | |
| **erase** | no | |
| **swap** (member) | no | Free `swap` partial |
| **clear** | yes | |
| **get_allocator** | no | |
| Relational ops / non-member **swap** | no / partial | |
| Range-for | yes | |

### `vector<bool>`

Ref: [vector\<bool\>](https://cplusplus.com/reference/vector/vector-bool/).

| Item | Status | Notes |
|------|--------|--------|
| Specialization (proxy refs, pack, `flip`, …) | **no** | No dedicated type; `vector` of `bool` is not bit-packed |

### `deque`

Ref: [deque](https://cplusplus.com/reference/deque/deque/).

| Item | Status | Notes |
|------|--------|--------|
| Container type | **no** | Sema may know the name; **no** heap object / methods |
| All members (`[]`, `push_front`/`back`, iterators, …) | **no** | |

### `list`

Ref: [list](https://cplusplus.com/reference/list/list/).

| Item | Status | Notes |
|------|--------|--------|
| Container type | **no** | Sema may know some members; **no** runtime |
| All members (`splice`, `merge`, `sort`, …) | **no** | |

### `array`

Ref: [array](https://cplusplus.com/reference/array/array/).

| Item | Status | Notes |
|------|--------|--------|
| Container type | **no** | Seeded in sema only; **no** runtime |
| All members (`fill`, `at`, `data`, …) | **no** | |

### `stack`

Ref: [stack](https://cplusplus.com/reference/stack/stack/). Runtime: `stl/stack.rs`.

| Member | Status | Notes |
|--------|--------|--------|
| **(constructor)** | partial | Default empty |
| **empty** / **size** | yes | |
| **top** | yes | |
| **push** / **emplace** | yes | |
| **pop** | yes | |
| **swap** (member) | no | Free `swap` partial |
| Custom underlying `Container` | no | Fixed Vec storage |
| Relational ops / `uses_allocator` | no | |

### `queue`

Ref: [queue](https://cplusplus.com/reference/queue/queue/). Runtime: `stl/queue.rs`.

| Member | Status | Notes |
|--------|--------|--------|
| **(constructor)** | partial | Default empty |
| **empty** / **size** | yes | |
| **front** / **back** | yes | |
| **push** / **emplace** | yes | |
| **pop** | yes | |
| **swap** (member) | no | Free `swap` partial |
| Custom underlying `Container` | no | Fixed `VecDeque` |
| Relational ops / `uses_allocator` | no | |

### `priority_queue`

Ref: [priority_queue](https://cplusplus.com/reference/queue/priority_queue/). Runtime: `stl/priority_queue.rs`.

| Member | Status | Notes |
|--------|--------|--------|
| **(constructor)** | partial | Default empty; no range/cmp ctors |
| **empty** / **size** | yes | |
| **top** | yes | |
| **push** / **emplace** | yes | **ints only** |
| **pop** | yes | |
| **swap** (member) | no | Free `swap` partial |
| Custom `Compare` / non-int `T` / underlying container | **no** | Max-heap of `i64` |
| Non-member **swap** / `uses_allocator` | no / partial | |

### `map`

Ref: [map](https://cplusplus.com/reference/map/map/). Runtime: `stl/map.rs` (shared with `unordered_map`) + `[]` in engine.

| Member | Status | Notes |
|--------|--------|--------|
| **(constructor)** | partial | Default / brace `{{k,v},…}`; no custom Compare |
| **(destructor)** | yes | |
| **operator=** | partial | |
| **begin** / **end** / **cbegin** / **cend** | partial | Stubs for `find != end` |
| **rbegin** / **rend** / **crbegin** / **crend** | no | |
| **empty** / **size** | yes | |
| **max_size** | no | |
| **operator[]** | yes | Default-insert mapped type |
| **at** | no | |
| **insert** / **emplace** | yes | Pair or `(k,v)`; not full overloads |
| **emplace_hint** | no | |
| **erase** | partial | By key only |
| **swap** (member) | no | Free `swap` partial |
| **clear** | yes | |
| **key_comp** / **value_comp** | no | |
| **find** | partial | Presence stub vs `end`; no `it->second` |
| **count** | yes | |
| **lower_bound** / **upper_bound** / **equal_range** | no | |
| **get_allocator** | no | |
| Relational ops / non-member **swap** | no / partial | |
| Range-for | yes | Pair `Alloc` or structured binding |
| Custom Compare / Alloc | no | Ordered via `BTreeMap` keys |

### `unordered_map`

Ref: [unordered_map](https://cplusplus.com/reference/unordered_map/unordered_map/). Same runtime as `map`.

| Member | Status | Notes |
|--------|--------|--------|
| **(constructor)** | partial | Default / brace init; no Hash/Pred/Alloc |
| **(destructor)** | yes | |
| **operator=** | partial | |
| **empty** / **size** | yes | |
| **max_size** | no | |
| **begin** / **end** / **cbegin** / **cend** | partial | Stubs |
| **operator[]** | yes | Default-insert |
| **at** | no | |
| **find** | partial | Presence stub; no `it->second` |
| **count** | yes | |
| **equal_range** | no | |
| **emplace** / **insert** | yes | Pair or `(k,v)` |
| **emplace_hint** | no | |
| **erase** | partial | By key |
| **clear** | yes | |
| **swap** (member) | no | Free `swap` partial |
| **bucket_count** / **max_bucket_count** / **bucket_size** / **bucket** | no | |
| **load_factor** / **max_load_factor** / **rehash** / **reserve** | no | |
| **hash_function** / **key_eq** / **get_allocator** | no | |
| Relational ops / non-member **swap** | no / partial | |
| Range-for | yes | |
| `unordered_multimap` | no | |

### `set`

Ref: [set](https://cplusplus.com/reference/set/set/). Runtime: `stl/set.rs` (shared with `unordered_set`).

| Member | Status | Notes |
|--------|--------|--------|
| **(constructor)** | partial | Default / brace keys; no custom Compare |
| **(destructor)** | yes | |
| **operator=** | partial | |
| **begin** / **end** / **cbegin** / **cend** | partial | Stubs |
| **rbegin** / **rend** / **crbegin** / **crend** | no | |
| **empty** / **size** | yes | |
| **max_size** | no | |
| **insert** / **emplace** | yes | Single key |
| **emplace_hint** | no | |
| **erase** | partial | By key |
| **swap** (member) | no | Free `swap` partial |
| **clear** | yes | |
| **key_comp** / **value_comp** | no | |
| **find** | partial | Presence stub |
| **count** | yes | |
| **lower_bound** / **upper_bound** / **equal_range** | no | |
| **get_allocator** | no | |
| Relational ops / non-member **swap** | no / partial | |
| Range-for | yes | |
| `multiset` | no | |

### `unordered_set`

Ref: [unordered_set](https://cplusplus.com/reference/unordered_set/unordered_set/). Same runtime as `set`.

| Member | Status | Notes |
|--------|--------|--------|
| **(constructor)** | partial | Default / brace keys; no Hash/Pred |
| **(destructor)** | yes | |
| **operator=** | partial | |
| **empty** / **size** | yes | |
| **max_size** | no | |
| **begin** / **end** / **cbegin** / **cend** | partial | Stubs |
| **find** | partial | Presence stub |
| **count** | yes | |
| **equal_range** | no | |
| **emplace** / **insert** | yes | Single key |
| **emplace_hint** | no | |
| **erase** | partial | By key |
| **clear** | yes | |
| **swap** (member) | no | Free `swap` partial |
| **bucket_*** / hash policy / observers | no | |
| Relational ops / non-member **swap** | no / partial | |
| Range-for | yes | |
| `unordered_multiset` | no | |

### `string`

| Op | Status |
|----|--------|
| `size` / `length`, `empty`, `clear`, `[]` | yes |
| `push_back`, `append`, `substr`, `find` | yes |
| `front` / `back`, begin/end stubs | yes / partial |
| `rfind`, `replace`, `compare`, `npos` full | no / partial |

### `pair`

| Op | Status |
|----|--------|
| `.first` / `.second`, brace / `pair(a,b)` | yes |
| Methods | no |

---

## Algorithms (`<algorithm>`)

Ref: [cplusplus.com `<algorithm>`](https://cplusplus.com/reference/algorithm/).  
**Cmp / pred** = optional lambda/closure overload (`cmp(a,b)→bool` or unary pred). Functors / function pointers → **no**.  
Ranges: `v.begin()`/`v.end()` or free `begin`/`end` on **vector** (string where noted). Returns for bounds are **index `Int`**, not real iterators.

### Non-modifying sequence

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `all_of` | no | no | |
| `any_of` | no | no | |
| `none_of` | no | no | |
| `for_each` | no | no | |
| `find` | no | n/a | |
| `find_if` / `find_if_not` | no | no | |
| `find_end` | no | no | |
| `find_first_of` | no | no | |
| `adjacent_find` | no | no | |
| `count` | no | n/a | |
| `count_if` | no | no | |
| `mismatch` | no | no | |
| `equal` | no | no | |
| `is_permutation` | no | no | |
| `search` / `search_n` | no | no | |

### Modifying sequence

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `copy` / `copy_n` / `copy_if` / `copy_backward` | no | no (`copy_if`) | |
| `move` / `move_backward` (ranges) | no | n/a | **Not** `std::move(x)` cast — that is **yes** (identity) under utilities |
| `swap` | yes | n/a | Free `swap`/`std::swap` on lvalues |
| `swap_ranges` / `iter_swap` | no | n/a | |
| `transform` | no | no | |
| `replace` / `replace_if` / `replace_copy*` | no | no | |
| `fill` / `fill_n` | no | n/a | |
| `generate` / `generate_n` | no | no | |
| `remove` / `remove_if` / `remove_copy*` | no | no | |
| `unique` / `unique_copy` | no | no | |
| `reverse` | yes | n/a | Vector (and string via range) |
| `reverse_copy` | no | n/a | |
| `rotate` / `rotate_copy` | no | n/a | |
| `random_shuffle` / `shuffle` | no | no | |

### Partitions

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `is_partitioned` | no | no | |
| `partition` / `stable_partition` | no | no | |
| `partition_copy` / `partition_point` | no | no | |

### Sorting

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `sort` | yes | **yes** | Int asc default; lambda cmp |
| `stable_sort` | no | no | |
| `partial_sort` / `partial_sort_copy` | no | no | |
| `is_sorted` / `is_sorted_until` | no | no | |
| `nth_element` | no | no | |

### Binary search (sorted ranges)

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `lower_bound` | yes | **yes** | Returns index `Int` |
| `upper_bound` | yes | **yes** | Returns index `Int` |
| `equal_range` | no | no | |
| `binary_search` | yes | **yes** | Returns `Bool` |

### Merge / set ops (sorted ranges)

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `merge` / `inplace_merge` | no | no | |
| `includes` | no | no | |
| `set_union` / `set_intersection` | no | no | |
| `set_difference` / `set_symmetric_difference` | no | no | |

### Heap

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `push_heap` / `pop_heap` / `make_heap` / `sort_heap` | no | no | |
| `is_heap` / `is_heap_until` | no | no | |

### Min / max

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `min` / `max` (two values) | yes | **no** | Ints via builtins; no cmp / init-list |
| `minmax` | no | no | |
| `min_element` | yes | **yes** | → `Ptr` Index |
| `max_element` | yes | **yes** | → `Ptr` Index |
| `minmax_element` | no | no | |

### Other

| Algo | Default | Cmp / pred | Notes |
|------|---------|------------|--------|
| `lexicographical_compare` | no | no | |
| `next_permutation` / `prev_permutation` | no | no | |

---

## `<numeric>` & other utilities (not `<algorithm>`, but related)

| Name | Default | Op / cmp | Notes |
|------|---------|----------|--------|
| `accumulate` | yes | **yes** | Binary op lambda; vector or string chars |
| `iota` | yes | n/a | |
| `partial_sum` | yes | no | In-place when out==begin |
| `std::move` (cast) | yes | n/a | Identity; not range `move` |
| `pow` / `sqrt` / `ceil` / `floor` / `abs` | yes | n/a | |
| `__builtin_popcount` | yes | n/a | |
| `tie` / `make_tuple` / `make_pair` | partial | n/a | Common LeetCode forms |

---

## Events / WASM

| Item | Status | Notes |
|------|--------|--------|
| Event stream (`docs/events.md`) | yes | |
| Loop lifecycle (`LoopIter` / `Continue` / `Break` / `LoopEnd`) | yes | |
| `call_id` only on `FnEnter`/`FnExit` | yes | |
| Bulk `ContainerMod.elems` (`sort`/`reverse`/`iota`/`partial_sum`) | yes | After-state snapshot |
| `run_method` (wasm / CLI) | yes | |
| Full VM parity with tree-walker | partial | Prefer tree-walker for new STL |

---

## Deliberately out / later

- Full C++ standard library
- True iterators / `it->second` / `it++`
- `multimap` / `multiset` / `unordered_multimap` / `unordered_multiset`
- Threads, I/O streams, filesystem

When something moves from **no** → **yes**/**partial**, update this file in the same change.
