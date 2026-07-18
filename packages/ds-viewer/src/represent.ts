import type { HeapSnapshot, ObjectState, ValueJson } from "@rscpp/timeline";

export type Representation =
  | "array"
  | "table"
  | "stack"
  | "queue"
  | "matrix"
  | "adjacency-list"
  | "adjacency-matrix"
  | "edge-list"
  | "tree"
  | "raw";

/** Graph subcategory encodings (selectable under Graph optgroup). */
export type GraphEncoding =
  | "adjacency-list"
  | "adjacency-matrix"
  | "edge-list";

export const GRAPH_ENCODINGS: GraphEncoding[] = [
  "adjacency-list",
  "adjacency-matrix",
  "edge-list",
];

export function isGraphEncoding(r: Representation): r is GraphEncoding {
  return (GRAPH_ENCODINGS as string[]).includes(r);
}

/** Display label in the select. */
export function representationLabel(r: Representation): string {
  switch (r) {
    case "adjacency-list":
      return "adjacency list";
    case "adjacency-matrix":
      return "adjacency matrix";
    case "edge-list":
      return "edge list";
    default:
      return r;
  }
}

/** Map legacy prefs to current ids. */
export function normalizeRepresentation(r: string): Representation | null {
  const map: Record<string, Representation> = {
    graph: "adjacency-list",
    adjacency: "adjacency-list",
    "edge-list": "edge-list",
    "adjacency-list": "adjacency-list",
    "adjacency-matrix": "adjacency-matrix",
    array: "array",
    table: "table",
    stack: "stack",
    queue: "queue",
    matrix: "matrix",
    tree: "tree",
    raw: "raw",
  };
  return map[r] ?? null;
}

function isObjectVal(v: ValueJson | undefined): v is { kind: "Object"; value: number } {
  return !!v && v.kind === "Object";
}

function isIntVal(v: ValueJson | undefined): v is { kind: "Int"; value: number } {
  return !!v && v.kind === "Int";
}

function rowInts(
  obj: ObjectState,
  heap: HeapSnapshot,
): number[][] | null {
  const elems = obj.elems ?? [];
  if (elems.length === 0 || !elems.every(isObjectVal)) return null;
  const rows: number[][] = [];
  for (const e of elems) {
    const child = heap.objects.get(e.value);
    const cells = child?.elems ?? [];
    if (!cells.every(isIntVal)) return null;
    rows.push(cells.map((c) => c.value));
  }
  return rows;
}

export function shapeFlags(obj: ObjectState, heap: HeapSnapshot): {
  adjList: boolean;
  adjMatrix: boolean;
  edgeList: boolean;
  intMatrix: boolean;
} {
  const rows = rowInts(obj, heap);
  const hasMapNeighbors =
    (obj.entries?.length ?? 0) > 0 || obj.type_name.includes("map");

  let edgeList = false;
  let adjList = false;
  let adjMatrix = false;
  let intMatrix = false;

  if (rows) {
    intMatrix = true;
    const n = rows.length;
    const square = rows.every((r) => r.length === n);
    adjMatrix = square && n > 0;
    // Neighbor lists: any row length; typical adj list is not forced-square 0/1
    adjList = rows.every((r) => r.every((v) => Number.isFinite(v)));
    edgeList = rows.every((r) => r.length === 2);
  }

  if (hasMapNeighbors) {
    // map / unordered_map keyed adjacency
    const entries = obj.entries ?? [];
    const ok =
      entries.length === 0 ||
      entries.every((ent) => {
        const val = ent.value;
        if (val?.kind !== "Object") return false;
        const neigh = heap.objects.get(val.value);
        return (neigh?.elems ?? []).every(isIntVal);
      });
    if (ok) adjList = true;
  }

  return { adjList, adjMatrix, edgeList, intMatrix };
}

export function proposeRepresentations(
  obj: ObjectState,
  heap: HeapSnapshot,
): Representation[] {
  const out: Representation[] = [];
  const elems = obj.elems ?? [];
  const entries = obj.entries ?? [];
  const shape = shapeFlags(obj, heap);

  if (entries.length > 0 || obj.type_name.includes("map")) {
    if (shape.adjList) out.push("adjacency-list");
    out.push("table", "raw");
    return unique(out);
  }

  // Default order: edge-list when every row is a pair; else adj-list before matrix.
  if (shape.edgeList) out.push("edge-list");
  if (shape.adjList) out.push("adjacency-list");
  if (shape.adjMatrix) out.push("adjacency-matrix");
  if (shape.intMatrix) out.push("matrix", "table");

  if (elems.length > 0 && elems.every(isIntVal)) {
    const n = elems.length;
    const parentsOk = elems.every((e) => e.value >= -1 && e.value < n);
    out.push("array", "table", "stack", "queue");
    if (parentsOk && n > 1) out.push("tree");
    out.push("raw");
    return unique(out);
  }

  if (out.length > 0) {
    out.push("raw");
    return unique(out);
  }

  if (elems.length >= 0) {
    out.push("array", "table", "stack", "queue", "raw");
  } else {
    out.push("raw");
  }
  return unique(out);
}

function unique(xs: Representation[]): Representation[] {
  return [...new Set(xs)];
}
