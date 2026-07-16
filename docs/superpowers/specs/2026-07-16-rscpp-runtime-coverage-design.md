# Runtime coverage hybrid (Valid Parentheses)

**Date:** 2026-07-16  
**Status:** approved  
**Goal:** First executable visualizer milestone beyond `twoSum`: run `Solution::isValid` via `run_method` with stack/string events; fill only the runtime gaps that path needs.

## Context

- Parse/sema corpus work made many LeetCode files typecheck; runtime already runs `Solution::twoSum`.
- Visualizer consumes the existing event contract (`docs/events.md`); no new event kinds for this milestone.
- Corpus file `testing/valid-parentheses.cpp` uses `unordered_map::find` + iterators — out of scope.

## Requirements

1. Curated `examples/valid_parentheses.cpp` with stack-based `isValid` (no map iterators).
2. `run_method(src, "Solution::isValid", ["()[]{}"])` → `true`; `["(]"]` → `false`.
3. Events include `FnEnter`/`FnExit` and `ContainerMod` for stack push/pop (and `emplace` as alias of push).
4. Runtime + wasm tests; tree-walker first (not VM parity).
5. Stretch if quick: `run_method` for `examples/dfs.cpp` `Solution::countComponents`.

## Non-goals

- Full corpus execute / golden I/O
- Map iterators (`find` / `cend` / `it->second`)
- TreeNode / islands demos
- New `Event` enum variants (new `op` strings on existing events OK)

## Design

- Example stays inside the supported subset: `stack<char>`, string range-for or indexing, char compares.
- `stack::emplace` → same semantics/events as `stack::push`.
- Prefer existing events; document any new `op` strings in `docs/events.md`.
