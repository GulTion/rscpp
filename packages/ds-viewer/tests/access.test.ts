import { describe, it, expect } from "vitest";
import type { HeapSnapshot } from "@rscpp/timeline";
import { accessHighlight } from "../src/access";

describe("accessHighlight", () => {
  it("tracks 1D index lookups and writes", () => {
    const heap: HeapSnapshot = {
      objects: new Map([
        [
          8,
          {
            type_name: "vector",
            elems: [
              { kind: "Int", value: 0 },
              { kind: "Int", value: 1 },
            ],
          },
        ],
      ]),
      frames: [],
      openLoops: [],
    };
    const events = [
      {
        kind: "ContainerLookup",
        op: "index",
        container: { kind: "Object", value: 8 },
        key: { kind: "Int", value: 0 },
        result: { kind: "Int", value: 0 },
      },
      { kind: "Step" },
      { kind: "Step" },
      { kind: "Step" },
      { kind: "Step" },
      {
        kind: "Write",
        slot: { kind: "Index", obj: 8, index: 1 },
        value: { kind: "Int", value: 1 },
      },
    ];
    const a = accessHighlight(events, events.length, 8, heap);
    expect(a.current).toContain(1);
    expect(a.trail).toContain(0);
  });

  it("tracks matrix cell via row object lookup", () => {
    const heap: HeapSnapshot = {
      objects: new Map([
        [
          6,
          {
            type_name: "vector",
            elems: [
              { kind: "Object", value: 0 },
              { kind: "Object", value: 1 },
            ],
          },
        ],
        [0, { type_name: "vector", elems: [{ kind: "Int", value: 1 }] }],
        [1, { type_name: "vector", elems: [{ kind: "Int", value: 2 }] }],
      ]),
      frames: [],
      openLoops: [],
    };
    const events = [
      {
        kind: "ContainerLookup",
        op: "index",
        container: { kind: "Object", value: 6 },
        key: { kind: "Int", value: 1 },
        result: { kind: "Object", value: 1 },
      },
      {
        kind: "ContainerLookup",
        op: "index",
        container: { kind: "Object", value: 1 },
        key: { kind: "Int", value: 0 },
        result: { kind: "Int", value: 2 },
      },
    ];
    const a = accessHighlight(events, 2, 6, heap);
    expect(a.currentCells).toContainEqual({ i: 1, j: 0 });
    expect(a.current).toContain(1);
  });
});
