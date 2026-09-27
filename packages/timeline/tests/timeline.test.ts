import { describe, it, expect } from "vitest";
import { createTimeline } from "../src/createTimeline";
import { snapPlayheadIndex } from "../src/silent";
import fixture from "../src/fixtures/vector_push.json";

describe("createTimeline", () => {
  it("seek emits tick with matching snapshot", () => {
    const tl = createTimeline({ events: fixture.events, source: "" });
    const ticks: number[] = [];
    tl.subscribe((e) => {
      if (e.type === "tick") ticks.push(e.index);
    });
    tl.seek(fixture.events.length);
    expect(tl.index).toBe(fixture.events.length);
    expect(tl.snapshot().objects.get(0)?.elems?.length).toBe(2);
    expect(ticks.at(-1)).toBe(fixture.events.length);
  });

  it("step skips silent kinds but still applies them", () => {
    const events = [
      { kind: "Step", span: { start: 0, end: 1 } },
      { kind: "Alloc", span: { start: 1, end: 2 } },
      { kind: "RefBind", span: { start: 2, end: 3 } },
      { kind: "Write", span: { start: 3, end: 4 } },
    ];
    const tl = createTimeline({ events, source: "" });
    tl.step(1);
    expect(tl.index).toBe(4);
    expect(events[tl.index - 1].kind).toBe("Write");
  });
});

describe("snapPlayheadIndex", () => {
  it("forwards off silent current event", () => {
    const events = [
      { kind: "Step" },
      { kind: "Alloc" },
      { kind: "Write" },
    ];
    expect(snapPlayheadIndex(events, 1)).toBe(3);
    expect(snapPlayheadIndex(events, 2)).toBe(3);
  });

  it("only first LoopIter per instance is visible", () => {
    const events = [
      { kind: "LoopIter", loop_id: 1 },
      { kind: "Write" },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "Write" },
      { kind: "LoopEnd", loop_id: 1 },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "Write" },
    ];
    // playhead after 2nd LoopIter (index 3) → snap forward to Write at 4
    expect(snapPlayheadIndex(events, 3)).toBe(4);
    // first LoopIter (index 1) stays
    expect(snapPlayheadIndex(events, 1)).toBe(1);
    // after LoopEnd, new first LoopIter (index 6) stays
    expect(snapPlayheadIndex(events, 6)).toBe(6);
  });
});
