import { describe, it, expect } from "vitest";
import { proposeRepresentations } from "../src/represent";
import { diffElems } from "../src/diff";
import type { HeapSnapshot } from "@rscpp/timeline";

const emptyHeap: HeapSnapshot = {
  objects: new Map(),
  frames: [],
  openLoops: [],
};

describe("proposeRepresentations", () => {
  it("vector of ints → array/table/stack/queue", () => {
    const obj = { type_name: "vector", elems: [{ kind: "Int" as const, value: 1 }] };
    expect(proposeRepresentations(obj, emptyHeap)).toContain("array");
  });

  it("vector of Object rows → matrix candidate", () => {
    const heap: HeapSnapshot = {
      objects: new Map([
        [1, { type_name: "vector", elems: [{ kind: "Int", value: 0 }] }],
      ]),
      frames: [],
      openLoops: [],
    };
    const obj = {
      type_name: "vector",
      elems: [{ kind: "Object" as const, value: 1 }],
    };
    expect(proposeRepresentations(obj, heap)).toContain("matrix");
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
