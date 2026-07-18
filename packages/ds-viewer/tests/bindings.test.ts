import { describe, it, expect } from "vitest";
import type { HeapSnapshot } from "@rscpp/timeline";
import { bindingsFromSnapshot, objectIdOf } from "../src/bindings";

describe("objectIdOf", () => {
  it("reads Object and Heap Ref", () => {
    expect(objectIdOf({ kind: "Object", value: 3 })).toBe(3);
    expect(
      objectIdOf({ kind: "Ref", value: { kind: "Heap", value: 7 } }),
    ).toBe(7);
    expect(objectIdOf({ kind: "Int", value: 1 })).toBeNull();
  });
});

describe("bindingsFromSnapshot", () => {
  it("only includes named live objects; title is variable name", () => {
    const snap: HeapSnapshot = {
      objects: new Map([
        [0, { type_name: "vector", elems: [] }],
        [1, { type_name: "vector", elems: [] }],
        [99, { type_name: "orphan", elems: [] }],
      ]),
      frames: [
        {
          call_id: 1,
          name: "main",
          parent_id: null,
          locals: new Map([
            ["nums", { kind: "Ref", value: { kind: "Heap", value: 0 } }],
            ["i", { kind: "Int", value: 0 }],
            ["seen", { kind: "Object", value: 1 }],
          ]),
        },
      ],
      openLoops: [],
    };
    const b = bindingsFromSnapshot(snap);
    expect(b.map((x) => x.title)).toEqual(["nums", "seen"]);
    expect(b.map((x) => x.id)).toEqual([0, 1]);
  });
});
