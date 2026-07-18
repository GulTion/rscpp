# DS Viewer graph walk highlights

Date: 2026-07-18  
Package: `@rscpp/ds-viewer`  
Status: approved for implementation

## Goal

While the playhead moves, graph panes show a **walk**: recent nodes fade as a trail, the current node(s) pulse, and the active edge is emphasized when `u → v` can be inferred.

## Signals

Relative to the displayed graph object id `G` and encoding:

- `ContainerLookup` / `Write` (Index) on `G` or its neighbor-row children
- **Adj list:** `G[u]` → node `u`; `row_u[i] → v` → nodes `u,v` + edge `u–v`
- **Adj matrix:** `G[i][j]` nonzero read/write → edge `i–j`
- **Edge list:** pair `[u,v]` lookup → edge `u–v`

## Visual

- Current nodes: strong accent fill + short CSS pulse
- Trail: last ~8 node ids before current, fading opacity
- Current edge: thicker accent stroke (arrow if Directed)
- Trail edges: thinner, lower opacity
- Updates on timeline tick/seek only

## Non-goals

- Inferring walk from unrelated locals (`visited`) unless that object is the shown graph
- Separate RAF animation loop beyond CSS transitions
