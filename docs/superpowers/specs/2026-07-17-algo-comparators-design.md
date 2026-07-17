# Algorithm comparator overloads

**Date:** 2026-07-17  
**Status:** approved  
**Goal:** Optional lambda comparators for `lower_bound`, `upper_bound`, `binary_search`, `min_element`, `max_element` (same style as `sort`).

## Semantics

Comparator is a closure `cmp(a, b) → bool` (“`a` ordered before `b`”). No free function pointers / functors.

| Algo | Default | With cmp |
|------|---------|----------|
| `lower_bound(b,e,x[,cmp])` | int `>=` | first `i` with `!cmp(v[i], x)` |
| `upper_bound(b,e,x[,cmp])` | int `>` | first `i` with `cmp(x, v[i])` |
| `binary_search(b,e,x[,cmp])` | int `==` | exists with `!cmp(v[i],x) && !cmp(x,v[i])` |
| `min_element(b,e[,cmp])` | int min | argmin under `cmp` |
| `max_element(b,e[,cmp])` | int max | argmax: prefer `cand` when `cmp(best, cand)` |

Returns unchanged: bound → index `Int`; search → `Bool`; min/max → `Ptr` Index.

## Out of scope

`stable_sort`, `partition`, `nth_element`, `equal_range`, real iterators.
