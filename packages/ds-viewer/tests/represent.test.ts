import { describe, it, expect } from "vitest";
import { proposeRepresentations } from "../src/represent";
import { edgesFromObject } from "../src/views/misc";
import { diffElems } from "../src/diff";
import type { HeapSnapshot } from "@rscpp/timeline";

const emptyHeap: HeapSnapshot = {
  objects: new Map(),
  frames: [],
  openLoops: [],
};

function heapWithRows(rows: number[][]): {
  obj: { type_name: string; elems: { kind: "Object"; value: number }[] };
  heap: HeapSnapshot;
} {
  const objects = new Map();
  const elems: { kind: "Object"; value: number }[] = [];
  rows.forEach((row, i) => {
    const id = i + 1;
    objects.set(id, {
      type_name: "vector",
      elems: row.map((v) => ({ kind: "Int" as const, value: v })),
    });
    elems.push({ kind: "Object", value: id });
  });
  return {
    obj: { type_name: "vector", elems },
    heap: { objects, frames: [], openLoops: [] },
  };
}

describe("proposeRepresentations", () => {
  it("vector of ints → array/table/stack/queue", () => {
    const obj = { type_name: "vector", elems: [{ kind: "Int" as const, value: 1 }] };
    expect(proposeRepresentations(obj, emptyHeap)).toContain("array");
  });

  it("neighbor lists → adjacency-list (not edge-list when jagged)", () => {
    const { obj, heap } = heapWithRows([
      [1, 2],
      [0],
      [0, 1],
    ]);
    const opts = proposeRepresentations(obj, heap);
    expect(opts).toContain("adjacency-list");
    expect(opts).not.toContain("edge-list");
    expect(opts).not.toContain("adjacency-matrix");
  });

  it("square 0/1 → both adj-list and adj-matrix; list preferred", () => {
    const { obj, heap } = heapWithRows([
      [0, 1],
      [1, 0],
    ]);
    const opts = proposeRepresentations(obj, heap);
    expect(opts).toContain("adjacency-list");
    expect(opts).toContain("adjacency-matrix");
    expect(opts.indexOf("adjacency-list")).toBeLessThan(
      opts.indexOf("adjacency-matrix"),
    );
  });

  it("pairs → edge-list first", () => {
    const { obj, heap } = heapWithRows([
      [0, 1],
      [1, 2],
      [2, 0],
    ]);
    const opts = proposeRepresentations(obj, heap);
    expect(opts[0]).toBe("edge-list");
    expect(opts).toContain("adjacency-list");
  });
});

describe("edgesFromObject encodings", () => {
  it("adjacency-list uses cell values as neighbors", () => {
    const { obj, heap } = heapWithRows([[1, 2], [0], [0]]);
    expect(edgesFromObject(obj, heap, "adjacency-list")).toEqual([
      { from: 0, to: 1 },
      { from: 0, to: 2 },
      { from: 1, to: 0 },
      { from: 2, to: 0 },
    ]);
  });

  it("adjacency-matrix uses nonzero column indices", () => {
    const { obj, heap } = heapWithRows([
      [0, 1, 0],
      [1, 0, 1],
      [0, 0, 0],
    ]);
    expect(edgesFromObject(obj, heap, "adjacency-matrix")).toEqual([
      { from: 0, to: 1 },
      { from: 1, to: 0 },
      { from: 1, to: 2 },
    ]);
  });

  it("edge-list reads pairs", () => {
    const { obj, heap } = heapWithRows([
      [3, 4],
      [4, 5],
    ]);
    expect(edgesFromObject(obj, heap, "edge-list")).toEqual([
      { from: 3, to: 4 },
      { from: 4, to: 5 },
    ]);
  });
});

describe("diffElems", () => {
  it("detects change", () => {
    const d = diffElems(
      { type_name: "vector", elems: [{ kind: "Int", value: 1 }] },
      { type_name: "vector", elems: [{ kind: "Int", value: 2 }] },
    );
    expect(d[0].kind).toBe("change");
  });
});
