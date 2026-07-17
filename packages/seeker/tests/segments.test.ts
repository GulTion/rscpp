import { describe, it, expect } from "vitest";
import { buildCallSegments, buildLoopSegments, colorIndexForName } from "../src/segments";

describe("buildCallSegments", () => {
  it("nests child call inside parent", () => {
    const events = [
      { kind: "FnEnter", name: "main", call_id: 0, parent_id: null },
      { kind: "Step" },
      { kind: "FnEnter", name: "fn", call_id: 1, parent_id: 0 },
      { kind: "Step" },
      { kind: "FnExit", name: "fn", call_id: 1, parent_id: 0 },
      { kind: "FnExit", name: "main", call_id: 0, parent_id: null },
    ];
    const segs = buildCallSegments(events);
    expect(segs).toHaveLength(2);
    const main = segs.find((s) => s.name === "main")!;
    const fn = segs.find((s) => s.name === "fn")!;
    expect(main.depth).toBe(0);
    expect(fn.depth).toBe(1);
    expect(fn.parent_id).toBe(0);
    expect(main.startIndex).toBe(0);
    expect(main.endIndex).toBe(5);
    expect(fn.startIndex).toBe(2);
    expect(fn.endIndex).toBe(4);
  });
});

describe("buildLoopSegments", () => {
  it("pairs loop segments with nesting depth", () => {
    const events = [
      { kind: "Step" },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "LoopIter", loop_id: 2 },
      { kind: "Step" },
      { kind: "LoopEnd", loop_id: 2, reason: "exhausted" },
      { kind: "LoopEnd", loop_id: 1, reason: "exhausted" },
    ];
    const segs = buildLoopSegments(events);
    const outer = segs.find((s) => s.loop_id === 1)!;
    const inner = segs.find((s) => s.loop_id === 2)!;
    expect(outer.depth).toBe(0);
    expect(inner.depth).toBe(1);
    expect(outer.startIndex).toBe(1);
    expect(outer.endIndex).toBe(5);
    expect(inner.startIndex).toBe(2);
    expect(inner.endIndex).toBe(4);
  });
});

describe("colorIndexForName", () => {
  it("is stable for same name", () => {
    expect(colorIndexForName("main", 7)).toBe(colorIndexForName("main", 7));
    expect(colorIndexForName("main", 7)).not.toBe(colorIndexForName("fn", 7));
  });
});
