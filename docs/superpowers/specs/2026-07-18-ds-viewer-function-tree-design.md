# DS Viewer Function Tree

Date: 2026-07-18  
Package: `@rscpp/ds-viewer`  
Status: approved for implementation

## Goal

A draggable **Function Tree** card on the DS canvas showing the live call tree with argument values. For `dfs(u, adj&, vis&)`, each activation shows `u` changing while `adj` / `vis` keep the same `#id`.

## Data

- Nodes from `FnEnter` / `FnExit` (`call_id`, `parent_id`, `name`, `args[]`).
- Param names: ordered `VarCreate` names after each `FnEnter` until `ScopeEnter` / next `Fn*`.
- At playhead `t`: include enters with `enterIndex < t`; active if not exited before `t`.

## Visual

- SVG tree (d3-hierarchy); label `shortName(a=…, b=#id)`.
- Active stack frames highlighted; exited dimmed.
- Arg that matches parent’s same name + same object id: muted; scalar/id change vs parent: accent.

## Placement

Fixed canvas pane (id `-1`), same drag/pack/reset as Alloc panes.
