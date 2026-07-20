import { describe, it, expect } from "vitest";
import {
  buildCallSegments,
  buildLoopSegments,
  colorIndexForName,
  currentSegment,
} from "../src/segments";

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

describe("currentSegment", () => {
  const events = [
    { kind: "FnEnter", name: "twoSum", call_id: 0, parent_id: null },
    { kind: "Write" },
    { kind: "LoopIter", loop_id: 1 },
    { kind: "Write" },
    { kind: "LoopEnd", loop_id: 1, reason: "exhausted" },
    { kind: "Write" },
    { kind: "FnExit", name: "twoSum", call_id: 0, parent_id: null },
  ];

  it("picks call when playhead is in function but outside loop", () => {
    // after first Write (index 2), before LoopIter applied as current focus inside fn
    const seg = currentSegment(events, 2);
    expect(seg?.kind).toBe("call");
    expect(seg?.label).toBe("twoSum");
  });

  it("picks loop when playhead is inside loop", () => {
    // after LoopIter+Write (index 4) still before LoopEnd fully past? index 4 = after LoopEnd event at 4
    // playhead 4: events[0..4) applied, current is LoopEnd — contains: start 2, end 4, p<=5 → yes loop
    // playhead 3: after Write inside loop
    const seg = currentSegment(events, 3);
    expect(seg?.kind).toBe("loop");
    expect(seg?.label).toBe("loop #1");
  });

  it("picks nested call over an outer loop that wraps it", () => {
    // countComponents while wraps dfs — detail should zoom to dfs, not the while
    const nested = [
      { kind: "FnEnter", name: "countComponents", call_id: 0, parent_id: null },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "FnEnter", name: "dfs", call_id: 1, parent_id: 0 },
      { kind: "Write" },
      { kind: "FnExit", name: "dfs", call_id: 1, parent_id: 0 },
      { kind: "LoopEnd", loop_id: 1, reason: "exhausted" },
      { kind: "FnExit", name: "countComponents", call_id: 0, parent_id: null },
    ];
    const seg = currentSegment(nested, 4); // inside dfs body
    expect(seg?.kind).toBe("call");
    expect(seg?.label).toBe("dfs");
  });

  it("returns null outside all segments", () => {
    expect(currentSegment(events, 0)).toBeNull();
  });
});
