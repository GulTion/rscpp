# DS Viewer graph encodings

Date: 2026-07-18  
Package: `@rscpp/ds-viewer`  
Status: approved for implementation

## Problem

Graph drawing guessed container layout. `vector<vector<int>>` was always read as an adjacency **matrix** (column index = neighbor), which breaks real **adjacency lists** (cell value = neighbor). Users need explicit encodings.

## UX

- **Graph** is a category (optgroup), not a selectable value.
- Selectable graph subcategories only:
  - `adjacency list`
  - `adjacency matrix`
  - `edge list`
- Other representations (`array`, `table`, `stack`, `queue`, `matrix` table, `tree`, `raw`) remain flat options.
- User may pick only one subcategory at a time; that choice drives edge extraction + SVG graph.

## Encodings

| Id | Label | Shape | Edges |
|----|-------|-------|-------|
| `adjacency-list` | adjacency list | `vector<vector<int>>` (row = neighbors) or map→vector of ints | For each row/key `u`, each int `v` → `u→v` |
| `adjacency-matrix` | adjacency matrix | `vector<vector<int>>` cells | `cell[i][j] ≠ 0` → `i→j` |
| `edge-list` | edge list | vector of pairs / length-2 int rows | Each `[u,v]` → `u→v` |

## Proposal rules

- Offer an encoding only when the heap shape matches.
- If both list and matrix fit the same object, offer both; **default preference: adjacency list** (LeetCode-common).
- Map/unordered_map of neighbor vectors → `adjacency-list` (+ table/raw).
- Remove bare `graph` / old `adjacency` / `edge-list` string aliases in favor of the three ids above (migrate prefs: map old → new).

## API / render

- `proposeRepresentations` returns the new ids among candidates.
- `edgesFromObject(obj, heap, encoding)` — no multi-heuristic fallback inside one call.
- `renderGraph(host, obj, heap, encoding)` uses that encoding.
- Pane `<select>` uses `<optgroup label="Graph">` for the three encodings.

## Non-goals

- Weighted edges UI, undirected toggle, bipartite layout.
- Persisting encoding across page reloads.
