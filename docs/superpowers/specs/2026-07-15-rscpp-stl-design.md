# rscpp STL Design

**Date:** 2026-07-15  
**Location:** `rscpp-runtime` (`value::Object` + method dispatch)  
**Phase:** 6

## Approach

LeetCode STL is implemented as heap `Object` variants and member/index operations inside the runtime (not a line-port of JSCPP includes). A separate `rscpp-stl` crate can be extracted later if WASM packaging needs it; for now one crate avoids splitting `Value`/`Object`.

## Containers

| Type | Storage | Ops |
|------|---------|-----|
| `vector` | `Vec<Value>` | `size`, `empty`, `push_back`, `pop_back`, `clear`, `[]` |
| `string` | `String` | `size`, `empty`, `clear`, `[]`, `+` (later) |
| `pair` | `{first,second}` | fields |
| `map` / `unordered_map` | ordered/hash map of `MapKey → Value` | `[]`, `insert`, `count`, `erase`, `size`, `empty`, `clear` |
| `set` / `unordered_set` | set of `MapKey` | `insert`, `count`, `erase`, `size`, `empty`, `clear` |
| `stack` | `Vec` | `push`, `pop`, `top`, `size`, `empty` |
| `queue` | `VecDeque` | `push`, `pop`, `front`, `back`, `size`, `empty` |
| `priority_queue` | binary heap (ints) | `push`, `pop`, `top`, `size`, `empty` |

## Keys

`MapKey`: `Int`, `Bool`, `Char`, `String` (from heap string objects). Enough for typical LeetCode maps.

## Events

Reuse `ContainerMod`, `Write` (map index via `Slot::Index` or field), `Alloc`.
