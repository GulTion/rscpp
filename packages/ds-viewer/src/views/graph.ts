import type { HeapSnapshot, ObjectState } from "@rscpp/timeline";
import { hierarchy, tree as d3tree } from "d3-hierarchy";
import dagre from "dagre";
import { edgesFromObject } from "./misc.js";
import type { GraphEncoding } from "../represent.js";
import {
  normalizeEdges,
  type GraphViewOpts,
  DEFAULT_GRAPH_OPTS,
} from "../graphOpts.js";
import type { WalkHighlight } from "../walk.js";
import type { AccessHighlight } from "../access.js";
import { EMPTY_ACCESS } from "../access.js";

const MAX_NODES = 200;
const NS = "http://www.w3.org/2000/svg";

const EMPTY_WALK: WalkHighlight = {
  currentNodes: [],
  trailNodes: [],
  currentEdges: [],
  trailEdges: [],
};

function ensureWalkStyle(): void {
  if (typeof document === "undefined") return;
  if (document.getElementById("ds-walk-style")) return;
  const style = document.createElement("style");
  style.id = "ds-walk-style";
  style.textContent = `
@keyframes ds-walk-pulse {
  0% { fill: #fbbf24; }
  50% { fill: #f59e0b; }
  100% { fill: #fbbf24; }
}
.ds-node-current { animation: ds-walk-pulse 0.55s ease-in-out infinite; stroke: #b45309; stroke-width: 2.5px; }
.ds-node-trail { fill: #fde68a; stroke: #d97706; opacity: 0.75; transition: opacity 0.25s, fill 0.25s; }
.ds-edge-current { stroke: #d97706; stroke-width: 2.8px; opacity: 1; }
.ds-edge-trail { stroke: #f59e0b; stroke-width: 2px; opacity: 0.45; }
`;
  document.head?.appendChild(style);
}

export function renderTree(
  host: HTMLElement,
  obj: ObjectState,
  _heap: HeapSnapshot,
  access: AccessHighlight = EMPTY_ACCESS,
): void {
  host.innerHTML = "";
  ensureWalkStyle();
  const cur = new Set(access.current.map(String));
  const trail = new Set(access.trail.map(String));
  const parents = (obj.elems ?? []).map((e) => (e.kind === "Int" ? e.value : -1));
  if (parents.length > MAX_NODES) {
    host.textContent = `tree too large (${parents.length} > ${MAX_NODES}); use table`;
    return;
  }

  type N = { id: number; children?: N[] };
  const nodes: N[] = parents.map((_, id) => ({ id }));
  let rootId = parents.findIndex((p) => p < 0);
  if (rootId < 0) rootId = 0;
  parents.forEach((p, i) => {
    if (p >= 0 && p < nodes.length && i !== p) {
      (nodes[p].children ??= []).push(nodes[i]);
    }
  });

  const root = hierarchy(nodes[rootId]);
  const layout = d3tree<N>().nodeSize([40, 60]);
  layout(root);

  const svg = document.createElementNS(NS, "svg");
  svg.setAttribute("data-testid", "ds-tree");
  let minX = Infinity,
    maxX = -Infinity,
    minY = Infinity,
    maxY = -Infinity;
  root.descendants().forEach((d) => {
    minX = Math.min(minX, d.x ?? 0);
    maxX = Math.max(maxX, d.x ?? 0);
    minY = Math.min(minY, d.y ?? 0);
    maxY = Math.max(maxY, d.y ?? 0);
  });
  const pad = 30;
  const w = maxX - minX + pad * 2;
  const h = maxY - minY + pad * 2;
  svg.setAttribute("width", String(Math.max(w, 120)));
  svg.setAttribute("height", String(Math.max(h, 80)));

  const g = document.createElementNS(NS, "g");
  g.setAttribute("transform", `translate(${pad - minX},${pad - minY})`);

  root.links().forEach((l) => {
    const line = document.createElementNS(NS, "line");
    line.setAttribute("x1", String(l.source.x ?? 0));
    line.setAttribute("y1", String(l.source.y ?? 0));
    line.setAttribute("x2", String(l.target.x ?? 0));
    line.setAttribute("y2", String(l.target.y ?? 0));
    line.setAttribute("stroke", "#64748b");
    g.appendChild(line);
  });

  root.descendants().forEach((d) => {
    const x = d.x ?? 0;
    const y = d.y ?? 0;
    const id = String(d.data.id);
    const c = document.createElementNS(NS, "circle");
    c.setAttribute("cx", String(x));
    c.setAttribute("cy", String(y));
    c.setAttribute("r", "12");
    c.setAttribute("fill", "#e0f2fe");
    c.setAttribute("stroke", "#0284c7");
    c.setAttribute("data-testid", `ds-node-${d.data.id}`);
    if (cur.has(id)) c.classList.add("ds-node-current");
    else if (trail.has(id)) c.classList.add("ds-node-trail");
    const t = document.createElementNS(NS, "text");
    t.setAttribute("x", String(x));
    t.setAttribute("y", String(y + 4));
    t.setAttribute("text-anchor", "middle");
    t.setAttribute("font-size", "11");
    t.textContent = String(d.data.id);
    g.append(c, t);
  });

  svg.appendChild(g);
  host.appendChild(svg);
}

function edgePath(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  index: number,
  count: number,
): string {
  if (x1 === x2 && y1 === y2) {
    const r = 18 + index * 6;
    return `M ${x1} ${y1 - 12} A ${r} ${r} 0 1 1 ${x1 + 0.1} ${y1 - 12}`;
  }
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.hypot(dx, dy) || 1;
  const nx = -dy / len;
  const ny = dx / len;
  const off = (index - (count - 1) / 2) * 10;
  const cx = (x1 + x2) / 2 + nx * off;
  const cy = (y1 + y2) / 2 + ny * off;
  // Shorten so arrow/line meets node circle (r≈12)
  const trim = 14;
  const sx = x1 + (dx / len) * trim;
  const sy = y1 + (dy / len) * trim;
  const ex = x2 - (dx / len) * trim;
  const ey = y2 - (dy / len) * trim;
  if (count === 1 && Math.abs(off) < 0.01) {
    return `M ${sx} ${sy} L ${ex} ${ey}`;
  }
  return `M ${sx} ${sy} Q ${cx} ${cy} ${ex} ${ey}`;
}

export function renderGraph(
  host: HTMLElement,
  obj: ObjectState,
  heap: HeapSnapshot,
  encoding: GraphEncoding,
  opts: GraphViewOpts = DEFAULT_GRAPH_OPTS,
  walk: WalkHighlight = EMPTY_WALK,
): void {
  host.innerHTML = "";
  ensureWalkStyle();
  const raw = edgesFromObject(obj, heap, encoding);
  const drawEdges = normalizeEdges(raw, opts);
  const currentNodes = new Set(walk.currentNodes.map(String));
  const trailNodes = new Set(walk.trailNodes.map(String));
  const currentEdgeKeys = new Set(
    walk.currentEdges.map((e) => `${e.from}->${e.to}`),
  );
  const trailEdgeKeys = new Set(walk.trailEdges.map((e) => `${e.from}->${e.to}`));
  // Undirected: also match reversed keys
  if (opts.direction === "undirected") {
    for (const e of walk.currentEdges) {
      currentEdgeKeys.add(`${e.to}->${e.from}`);
      currentEdgeKeys.add(`${Math.min(e.from, e.to)}->${Math.max(e.from, e.to)}`);
    }
    for (const e of walk.trailEdges) {
      trailEdgeKeys.add(`${e.to}->${e.from}`);
      trailEdgeKeys.add(`${Math.min(e.from, e.to)}->${Math.max(e.from, e.to)}`);
    }
  }
  const ids = new Set<number>();
  for (const e of drawEdges) {
    ids.add(e.from);
    ids.add(e.to);
  }
  if (encoding === "adjacency-list" || encoding === "adjacency-matrix") {
    const elems = obj.elems ?? [];
    if (elems.every((e) => e.kind === "Object")) {
      elems.forEach((_, i) => ids.add(i));
    }
  }
  if (ids.size === 0) {
    host.textContent = "no graph edges";
    return;
  }
  if (ids.size > MAX_NODES) {
    host.textContent = `graph too large (${ids.size} > ${MAX_NODES}); use table`;
    return;
  }

  // Layout on unique pairs (ignore parallel copies)
  const layoutEdges = new Map<string, { from: number; to: number }>();
  for (const e of drawEdges) {
    const k = `${e.from}->${e.to}`;
    if (!layoutEdges.has(k)) layoutEdges.set(k, { from: e.from, to: e.to });
  }

  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "TB", nodesep: 36, ranksep: 48 });
  g.setDefaultEdgeLabel(() => ({}));
  for (const id of ids) {
    g.setNode(String(id), { width: 28, height: 28, label: String(id) });
  }
  for (const e of layoutEdges.values()) {
    g.setEdge(String(e.from), String(e.to));
  }
  dagre.layout(g);

  const svg = document.createElementNS(NS, "svg");
  svg.setAttribute("data-testid", "ds-graph");
  svg.dataset.direction = opts.direction;
  svg.dataset.multigraph = String(opts.multigraph);
  const graph = g.graph();
  svg.setAttribute("width", String((graph.width ?? 200) + 40));
  svg.setAttribute("height", String((graph.height ?? 120) + 40));

  const arrowId = `ds-arrow-${Math.random().toString(36).slice(2, 9)}`;
  const defs = document.createElementNS(NS, "defs");
  const marker = document.createElementNS(NS, "marker");
  marker.setAttribute("id", arrowId);
  marker.setAttribute("viewBox", "0 0 10 10");
  marker.setAttribute("refX", "9");
  marker.setAttribute("refY", "5");
  marker.setAttribute("markerWidth", "7");
  marker.setAttribute("markerHeight", "7");
  marker.setAttribute("orient", "auto-start-reverse");
  const tip = document.createElementNS(NS, "path");
  tip.setAttribute("d", "M 0 0 L 10 5 L 0 10 z");
  tip.setAttribute("fill", "#64748b");
  marker.appendChild(tip);
  defs.appendChild(marker);
  svg.appendChild(defs);

  const layer = document.createElementNS(NS, "g");
  layer.setAttribute("transform", "translate(20,20)");

  for (const e of drawEdges) {
    const a = g.node(String(e.from));
    const b = g.node(String(e.to));
    if (!a || !b) continue;
    const path = document.createElementNS(NS, "path");
    path.setAttribute("d", edgePath(a.x, a.y, b.x, b.y, e.index, e.count));
    path.setAttribute("fill", "none");
    path.setAttribute("stroke", "#64748b");
    path.setAttribute("stroke-width", "1.5");
    path.dataset.testid = `ds-edge-${e.from}-${e.to}-${e.index}`;
    const ek = `${e.from}->${e.to}`;
    if (currentEdgeKeys.has(ek)) path.classList.add("ds-edge-current");
    else if (trailEdgeKeys.has(ek)) path.classList.add("ds-edge-trail");
    if (opts.direction === "directed") {
      path.setAttribute("marker-end", `url(#${arrowId})`);
    }
    layer.appendChild(path);
  }

  g.nodes().forEach((id) => {
    const n = g.node(id);
    const c = document.createElementNS(NS, "circle");
    c.setAttribute("cx", String(n.x));
    c.setAttribute("cy", String(n.y));
    c.setAttribute("r", "12");
    c.setAttribute("fill", "#fce7f3");
    c.setAttribute("stroke", "#db2777");
    c.setAttribute("data-testid", `ds-node-${id}`);
    if (currentNodes.has(id)) c.classList.add("ds-node-current");
    else if (trailNodes.has(id)) c.classList.add("ds-node-trail");
    const t = document.createElementNS(NS, "text");
    t.setAttribute("x", String(n.x));
    t.setAttribute("y", String(n.y + 4));
    t.setAttribute("text-anchor", "middle");
    t.setAttribute("font-size", "11");
    t.textContent = id;
    layer.append(c, t);
  });

  svg.appendChild(layer);
  host.appendChild(svg);
}
