import type { HeapSnapshot, ObjectState } from "@rscpp/timeline";
import { hierarchy, tree as d3tree } from "d3-hierarchy";
import dagre from "dagre";
import { edgesFromObject } from "./misc.js";

const MAX_NODES = 200;

export function renderTree(
  host: HTMLElement,
  obj: ObjectState,
  _heap: HeapSnapshot,
): void {
  host.innerHTML = "";
  const parents = (obj.elems ?? [])
    .map((e) => (e.kind === "Int" ? e.value : -1));
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

  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
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

  const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
  g.setAttribute("transform", `translate(${pad - minX},${pad - minY})`);

  root.links().forEach((l) => {
    const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
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
    const c = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    c.setAttribute("cx", String(x));
    c.setAttribute("cy", String(y));
    c.setAttribute("r", "12");
    c.setAttribute("fill", "#e0f2fe");
    c.setAttribute("stroke", "#0284c7");
    c.setAttribute("data-testid", `ds-node-${d.data.id}`);
    const t = document.createElementNS("http://www.w3.org/2000/svg", "text");
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

export function renderGraph(
  host: HTMLElement,
  obj: ObjectState,
  heap: HeapSnapshot,
): void {
  host.innerHTML = "";
  const edges = edgesFromObject(obj, heap);
  const ids = new Set<number>();
  edges.forEach((e) => {
    ids.add(e.from);
    ids.add(e.to);
  });
  // Include isolated adjacency rows (e.g. component {5})
  const elems = obj.elems ?? [];
  if (elems.every((e) => e.kind === "Object")) {
    elems.forEach((_, i) => ids.add(i));
  }
  if (ids.size === 0) {
    host.textContent = "no graph edges";
    return;
  }
  if (ids.size > MAX_NODES) {
    host.textContent = `graph too large (${ids.size} > ${MAX_NODES}); use table`;
    return;
  }

  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "TB", nodesep: 30, ranksep: 40 });
  g.setDefaultEdgeLabel(() => ({}));
  for (const id of ids) g.setNode(String(id), { width: 28, height: 28, label: String(id) });
  for (const e of edges) g.setEdge(String(e.from), String(e.to));
  dagre.layout(g);

  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("data-testid", "ds-graph");
  const graph = g.graph();
  svg.setAttribute("width", String((graph.width ?? 200) + 40));
  svg.setAttribute("height", String((graph.height ?? 120) + 40));

  const layer = document.createElementNS("http://www.w3.org/2000/svg", "g");
  layer.setAttribute("transform", "translate(20,20)");

  g.edges().forEach((e) => {
    const edge = g.edge(e);
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    const pts = (edge.points as { x: number; y: number }[])
      .map((p, i) => `${i === 0 ? "M" : "L"}${p.x},${p.y}`)
      .join(" ");
    path.setAttribute("d", pts);
    path.setAttribute("fill", "none");
    path.setAttribute("stroke", "#64748b");
    layer.appendChild(path);
  });

  g.nodes().forEach((id) => {
    const n = g.node(id);
    const c = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    c.setAttribute("cx", String(n.x));
    c.setAttribute("cy", String(n.y));
    c.setAttribute("r", "12");
    c.setAttribute("fill", "#fce7f3");
    c.setAttribute("stroke", "#db2777");
    c.setAttribute("data-testid", `ds-node-${id}`);
    const t = document.createElementNS("http://www.w3.org/2000/svg", "text");
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
