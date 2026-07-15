# rscpp Memory Model Design

**Date:** 2026-07-15  
**Crate:** `rscpp-runtime` (heap + address layer)  
**Phase:** 7

## Goals

- Explicit addresses for stack slots and heap objects / elements / fields.
- `Value::Ref` (non-reseating alias) and `Value::Ptr` (reseating).
- Wire `RefBind` / `PtrMove` / `Dealloc` events for the visualizer.
- `&expr`, `*ptr`, reference parameters and `T&` locals.

## Non-goals

- Full C++ UB / provenance / overlapping lifetime rules
- GC / cycle collection
- Segmentation of a byte-addressable memory (IDs + slots are enough for LeetCode viz)

## Model

```text
Address =
  Null
| Stack { depth, name }     // slot in a call frame
| Heap(ObjId)
| Index { obj, index }
| Field { obj, field }
| MapEntry { obj, key }
```

- Creating `int& r = x` stores `Ref(Stack{…x…})` and emits `RefBind`.
- `int* p = &x` stores `Ptr(…)` and emits `PtrMove`.
- Frame pop: naively `Dealloc` heap objects **owned** by locals that are plain `Object` handles (not Ptr/Ref). Returned objects are not freed.

## Why this vs JSCPP

JSCPP pointers are JS object wrappers. We keep a small address ADT and events the UI can render as arrows without scraping interpreter guts.
