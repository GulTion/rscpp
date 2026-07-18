import type { EventJson, HeapSnapshot, ValueJson } from "@rscpp/timeline";
import type { GraphEncoding } from "./represent.js";

export type WalkEdge = { from: number; to: number };

export type WalkHighlight = {
  currentNodes: number[];
  trailNodes: number[];
  currentEdges: WalkEdge[];
  trailEdges: WalkEdge[];
};

const TRAIL_NODES = 8;
const TRAIL_EDGES = 8;

function objId(v: unknown): number | null {
  if (!v || typeof v !== "object") return null;
  const o = v as { kind?: string; value?: unknown };
  if (o.kind === "Object" && typeof o.value === "number") return o.value;
  return null;
}

function intOf(v: unknown): number | null {
  if (!v || typeof v !== "object") return null;
  const o = v as ValueJson;
  if (o.kind === "Int" && typeof o.value === "number") return o.value;
  return null;
}

/** Map neighbor-row heap id → node index for adjacency-list / matrix. */
export function rowIndexMap(
  graphId: number,
  heap: HeapSnapshot,
): Map<number, number> {
  const map = new Map<number, number>();
  const obj = heap.objects.get(graphId);
  const elems = obj?.elems ?? [];
  elems.forEach((e, i) => {
    if (e.kind === "Object") map.set(e.value, i);
  });
  return map;
}

type Touch = { nodes: number[]; edges: WalkEdge[] };

function touchFromEvent(
  ev: EventJson,
  graphId: number,
  rows: Map<number, number>,
  encoding: GraphEncoding,
): Touch {
  const nodes: number[] = [];
  const edges: WalkEdge[] = [];

  if (ev.kind === "ContainerLookup" && ev.op === "index") {
    const cid = objId(ev.container);
    const key = intOf(ev.key);
    const result = ev.result as ValueJson | undefined;
    if (cid === graphId && key !== null) {
      nodes.push(key);
      if (encoding === "edge-list" && result?.kind === "Object") {
        // pair object — edge filled when pair elems are read; mark node keys later
      }
      if (
        encoding === "adjacency-matrix" &&
        result?.kind === "Int" &&
        result.value !== 0 &&
        key !== null
      ) {
        // single-level matrix unlikely; usually nested
      }
    }
    if (cid !== null && rows.has(cid) && key !== null) {
      const u = rows.get(cid)!;
      nodes.push(u);
      if (encoding === "adjacency-list" && result?.kind === "Int") {
        const v = result.value;
        nodes.push(v);
        edges.push({ from: u, to: v });
      }
      if (encoding === "adjacency-matrix" && result?.kind === "Int") {
        const j = key;
        if (result.value !== 0) {
          nodes.push(j);
          edges.push({ from: u, to: j });
        } else {
          nodes.push(j);
        }
      }
      if (encoding === "edge-list" && result?.kind === "Int") {
        // reading pair[0] or pair[1] — accumulate via separate path below
        nodes.push(result.value);
      }
    }
  }

  if (ev.kind === "Write") {
    const slot = ev.slot as { kind?: string; obj?: number; index?: number } | undefined;
    if (slot?.kind === "Index" && typeof slot.obj === "number") {
      if (slot.obj === graphId && typeof slot.index === "number") {
        nodes.push(slot.index);
      }
      if (rows.has(slot.obj) && typeof slot.index === "number") {
        const u = rows.get(slot.obj)!;
        nodes.push(u);
        const val = ev.value as ValueJson | undefined;
        if (encoding === "adjacency-list" && val?.kind === "Int") {
          nodes.push(val.value);
          edges.push({ from: u, to: val.value });
        }
        if (encoding === "adjacency-matrix" && val?.kind === "Int" && val.value !== 0) {
          nodes.push(slot.index);
          edges.push({ from: u, to: slot.index });
        }
      }
    }
  }

  return { nodes: [...new Set(nodes)], edges };
}

function edgeKey(e: WalkEdge): string {
  return `${e.from}->${e.to}`;
}

/**
 * Build walk highlight from events `[0, timelineIndex)`.
 * Current = latest touch; trail = prior unique nodes/edges (capped).
 */
export function walkHighlight(
  events: EventJson[],
  timelineIndex: number,
  graphId: number,
  heap: HeapSnapshot,
  encoding: GraphEncoding,
): WalkHighlight {
  const rows = rowIndexMap(graphId, heap);
  const t = Math.max(0, Math.min(timelineIndex, events.length));
  type Dated = Touch & { at: number };
  const seq: Dated[] = [];
  for (let i = 0; i < t; i++) {
    const touch = touchFromEvent(events[i], graphId, rows, encoding);
    if (touch.nodes.length || touch.edges.length) seq.push({ ...touch, at: i });
  }

  if (seq.length === 0) {
    return { currentNodes: [], trailNodes: [], currentEdges: [], trailEdges: [] };
  }

  // Merge a short burst at the end (adj[u] then adj[u][i] within 3 events)
  const last = seq[seq.length - 1];
  let cur: Touch = { nodes: [...last.nodes], edges: [...last.edges] };
  for (let j = seq.length - 2; j >= 0; j--) {
    if (seq[j].at < last.at - 3) break;
    cur = {
      nodes: [...new Set([...seq[j].nodes, ...cur.nodes])],
      edges: [...seq[j].edges, ...cur.edges],
    };
  }

  const currentNodeSet = new Set(cur.nodes);
  const currentEdgeSet = new Set(cur.edges.map(edgeKey));

  const trailNodes: number[] = [];
  const seenN = new Set<number>();
  for (let i = seq.length - 1; i >= 0 && trailNodes.length < TRAIL_NODES; i--) {
    for (const n of [...seq[i].nodes].reverse()) {
      if (currentNodeSet.has(n) || seenN.has(n)) continue;
      seenN.add(n);
      trailNodes.push(n);
      if (trailNodes.length >= TRAIL_NODES) break;
    }
  }
  trailNodes.reverse();

  const trailEdges: WalkEdge[] = [];
  const seenE = new Set<string>();
  for (let i = seq.length - 1; i >= 0 && trailEdges.length < TRAIL_EDGES; i--) {
    for (const e of [...seq[i].edges].reverse()) {
      const k = edgeKey(e);
      if (currentEdgeSet.has(k) || seenE.has(k)) continue;
      seenE.add(k);
      trailEdges.push(e);
      if (trailEdges.length >= TRAIL_EDGES) break;
    }
  }
  trailEdges.reverse();

  // Dedupe current edges
  const ce: WalkEdge[] = [];
  const ck = new Set<string>();
  for (const e of cur.edges) {
    const k = edgeKey(e);
    if (ck.has(k)) continue;
    ck.add(k);
    ce.push(e);
  }

  return {
    currentNodes: [...currentNodeSet],
    trailNodes,
    currentEdges: ce,
    trailEdges,
  };
}
