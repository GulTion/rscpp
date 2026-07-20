import { describe, it, expect } from "vitest";
import {
  buildCallSegments,
  buildLoopSegments,
} from "../src/segments";
import {
  LAYER_H,
  allSpans,
  callStackDepth,
  loopStackDepth,
  maxStackDepth,
  slabBottom,
  stackHeight,
} from "../src/stackLayout";
import dfs from "../../timeline/src/fixtures/dfs.json";

describe("stackLayout", () => {
  it("places deeper slabs higher via bottom offset", () => {
    expect(slabBottom(0)).toBe(0);
    expect(slabBottom(1)).toBe(LAYER_H);
    expect(slabBottom(2)).toBe(LAYER_H * 2);
    expect(stackHeight(2)).toBeGreaterThan(stackHeight(0));
  });

  it("stacks nested calls and loops by containment", () => {
    const events = [
      { kind: "FnEnter", name: "main", call_id: 0, parent_id: null },
      { kind: "Step" },
      { kind: "FnEnter", name: "fn", call_id: 1, parent_id: 0 },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "Step" },
      { kind: "LoopEnd", loop_id: 1, reason: "exhausted" },
      { kind: "FnExit", name: "fn", call_id: 1, parent_id: 0 },
      { kind: "FnExit", name: "main", call_id: 0, parent_id: null },
    ];
    const calls = buildCallSegments(events);
    const loops = buildLoopSegments(events);
    const spans = allSpans(calls, loops);
    const n = events.length;
    const main = calls.find((s) => s.name === "main")!;
    const fn = calls.find((s) => s.name === "fn")!;
    const loop = loops.find((s) => s.loop_id === 1)!;

    expect(callStackDepth(main, spans, n)).toBe(0);
    expect(callStackDepth(fn, spans, n)).toBe(1);
    // loop inside fn → above fn
    expect(loopStackDepth(loop, spans, n)).toBe(2);
    expect(maxStackDepth(calls, loops, n)).toBe(2);
  });

  it("orders dfs countComponents: loop between parent and child dfs", () => {
    const events = dfs.events as { kind: string }[];
    const calls = buildCallSegments(events);
    const loops = buildLoopSegments(events);
    const spans = allSpans(calls, loops);
    const n = events.length;

    const count = calls.find((c) => c.name.includes("countComponents"))!;
    const dfs0 = calls.find((c) => c.call_id === 1)!; // first root dfs
    const dfs1 = calls.find((c) => c.call_id === 2)!; // child of dfs0
    const loopInDfs0 = loops.find((l) => l.loop_id === 2)!; // while in dfs0
    const outerWhile = loops.find((l) => l.loop_id === 1)!; // while i < n

    const dCount = callStackDepth(count, spans, n);
    const dOuter = loopStackDepth(outerWhile, spans, n);
    const dDfs0 = callStackDepth(dfs0, spans, n);
    const dLoop = loopStackDepth(loopInDfs0, spans, n);
    const dDfs1 = callStackDepth(dfs1, spans, n);

    expect(dCount).toBe(0);
    // outer while wraps the dfs roots
    expect(dOuter).toBeGreaterThan(dCount);
    expect(dDfs0).toBeGreaterThan(dOuter);
    // loop inside dfs0 sits between dfs0 and recursive dfs1
    expect(dLoop).toBeGreaterThan(dDfs0);
    expect(dDfs1).toBeGreaterThan(dLoop);

    // sibling component roots share a layer
    const dfsRootA = calls.find((c) => c.call_id === 1)!;
    const dfsRootB = calls.find((c) => c.call_id === 4)!;
    expect(callStackDepth(dfsRootA, spans, n)).toBe(
      callStackDepth(dfsRootB, spans, n),
    );
  });
});
