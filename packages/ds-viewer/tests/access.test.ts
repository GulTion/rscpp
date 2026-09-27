import { describe, it, expect } from "vitest";
import type { HeapSnapshot } from "@rscpp/timeline";
import { accessHighlight } from "../src/access";

describe("accessHighlight", () => {
  it("marks reads yellow-set, writes green-set with old value only on process", () => {
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
      {
        kind: "Write",
        slot: { kind: "Index", obj: 8, index: 1 },
        old: { kind: "Int", value: 9 },
        value: { kind: "Int", value: 1 },
      },
    ];
    const a = accessHighlight(events, events.length, 8, heap);
    expect(a.process).toContain(1);
    expect(a.write).toContain(1);
    expect(a.read).toContain(0);
    expect(a.writeOld.get("1")).toBe("9");
    expect(a.animateKeys.has("1")).toBe(true);
    expect(a.indexHint?.key).toBeUndefined();
  });

  it("does not keep writeOld / animate after leaving the write event", () => {
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
        kind: "Write",
        slot: { kind: "Index", obj: 8, index: 1 },
        old: { kind: "Int", value: 9 },
        value: { kind: "Int", value: 1 },
      },
      { kind: "Step" },
      {
        kind: "ContainerLookup",
        op: "index",
        container: { kind: "Object", value: 8 },
        key: { kind: "Int", value: 0 },
        result: { kind: "Int", value: 0 },
      },
    ];
    const a = accessHighlight(events, events.length, 8, heap);
    expect(a.write).toContain(1);
    expect(a.writeOld.size).toBe(0);
    expect(a.animateKeys.size).toBe(0);
    expect(a.indexHint?.key).toBe("0");
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
    expect(a.processCells).toContainEqual({ i: 1, j: 0 });
    expect(a.process).toContain(1);
    expect(a.indexHint?.key).toBe("0");
  });

  it("marks push structural on last index", () => {
    const heap: HeapSnapshot = {
      objects: new Map([
        [
          0,
          {
            type_name: "vector",
            elems: [
              { kind: "Int", value: 1 },
              { kind: "Int", value: 2 },
            ],
          },
        ],
      ]),
      frames: [],
      openLoops: [],
    };
    const events = [
      {
        kind: "ContainerMod",
        op: "push_back",
        container: { kind: "Object", value: 0 },
        value: { kind: "Int", value: 2 },
      },
    ];
    const a = accessHighlight(events, 1, 0, heap);
    expect(a.structural).toEqual({ op: "push", index: 1 });
    expect(a.animateKeys.has("1")).toBe(true);
  });
});
