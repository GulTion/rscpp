import type { HeapSnapshot, ObjectState, ValueJson } from "@rscpp/timeline";

export type Representation =
  | "array"
  | "table"
  | "stack"
  | "queue"
  | "matrix"
  | "edge-list"
  | "adjacency"
  | "tree"
  | "graph"
  | "raw";

function isObjectVal(v: ValueJson | undefined): v is { kind: "Object"; value: number } {
  return !!v && v.kind === "Object";
}

function isIntVal(v: ValueJson | undefined): v is { kind: "Int"; value: number } {
  return !!v && v.kind === "Int";
}

export function proposeRepresentations(
  obj: ObjectState,
  heap: HeapSnapshot,
): Representation[] {
  const out: Representation[] = [];
  const elems = obj.elems ?? [];
  const entries = obj.entries ?? [];

  if (entries.length > 0 || obj.type_name.includes("map")) {
    out.push("adjacency", "table", "raw");
    return unique(out);
  }

  if (elems.length > 0 && elems.every(isObjectVal)) {
    const child = heap.objects.get(elems[0].value);
    if (child?.elems && child.elems.every((e) => e.kind === "Int" || e.kind === "Object")) {
      out.push("matrix", "edge-list", "graph", "table", "raw");
      return unique(out);
    }
  }

  if (elems.length > 0 && elems.every(isIntVal)) {
    const n = elems.length;
    const parentsOk = elems.every((e) => e.value >= -1 && e.value < n);
    out.push("array", "table", "stack", "queue");
    if (parentsOk && n > 1) out.push("tree");
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
