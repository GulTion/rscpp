import { describe, it, expect } from "vitest";
import type { HeapSnapshot } from "@rscpp/timeline";
import { walkHighlight, rowIndexMap } from "../src/walk";

function adjHeap(): { graphId: number; heap: HeapSnapshot } {
  // graph id 6 → rows 0,1,2 as objects 0,1,2
  const heap: HeapSnapshot = {
    objects: new Map([
      [
        6,
        {
          type_name: "vector",
          elems: [
            { kind: "Object", value: 0 },
            { kind: "Object", value: 1 },
            { kind: "Object", value: 2 },
          ],
        },
      ],
      [0, { type_name: "vector", elems: [{ kind: "Int", value: 1 }] }],
      [
        1,
        {
          type_name: "vector",
          elems: [
            { kind: "Int", value: 0 },
            { kind: "Int", value: 2 },
          ],
        },
      ],
      [2, { type_name: "vector", elems: [{ kind: "Int", value: 1 }] }],
    ]),
    frames: [],
    openLoops: [],
  };
  return { graphId: 6, heap };
}

describe("rowIndexMap", () => {
  it("maps row objects to node indices", () => {
    const { graphId, heap } = adjHeap();
    expect([...rowIndexMap(graphId, heap).entries()]).toEqual([
      [0, 0],
      [1, 1],
      [2, 2],
    ]);
  });
});

describe("walkHighlight", () => {
  it("highlights node + edge for adj[u][i] walk", () => {
    const { graphId, heap } = adjHeap();
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
        key: { kind: "Int", value: 1 },
        result: { kind: "Int", value: 2 },
      },
    ];
    const w = walkHighlight(events, 2, graphId, heap, "adjacency-list");
    expect(w.currentNodes.sort()).toEqual([1, 2]);
    expect(w.currentEdges).toEqual([{ from: 1, to: 2 }]);
  });

  it("keeps a trail of earlier nodes", () => {
    const { graphId, heap } = adjHeap();
    const events = [
      {
        kind: "ContainerLookup",
        op: "index",
        container: { kind: "Object", value: 6 },
        key: { kind: "Int", value: 0 },
        result: { kind: "Object", value: 0 },
      },
      { kind: "Step" },
      { kind: "Step" },
      { kind: "Step" },
      { kind: "Step" },
      {
        kind: "ContainerLookup",
        op: "index",
        container: { kind: "Object", value: 6 },
        key: { kind: "Int", value: 2 },
        result: { kind: "Object", value: 2 },
      },
    ];
    const w = walkHighlight(events, events.length, graphId, heap, "adjacency-list");
    expect(w.currentNodes).toContain(2);
    expect(w.trailNodes).toContain(0);
  });
});
