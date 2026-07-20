import { describe, it, expect } from "vitest";
import {
  countVisibleInRange,
  playheadToVisualInRange,
  visualToPlayheadInRange,
} from "../src/detailAxis";

describe("detailAxis", () => {
  // indices 2..4 are the "segment"; Step/Alloc silent; Write visible
  const events = [
    { kind: "FnEnter" }, // 0 visible
    { kind: "Step" }, // 1 silent
    { kind: "LoopIter", loop_id: 1 }, // 2 visible (first)
    { kind: "Alloc" }, // 3 silent
    { kind: "Write" }, // 4 visible
    { kind: "LoopIter", loop_id: 1 }, // 5 silent (repeat)
    { kind: "Write" }, // 6 visible
    { kind: "LoopEnd", loop_id: 1 }, // 7 visible
  ];

  it("counts only non-silent events inside the range", () => {
    // segment loop: 2..7 → LoopIter, Write, (silent LoopIter), Write, LoopEnd = 4
    expect(countVisibleInRange(events, 2, 7)).toBe(4);
  });

  it("maps playhead to local visual within range", () => {
    expect(playheadToVisualInRange(events, 2, 2, 7)).toBe(0);
    expect(playheadToVisualInRange(events, 3, 2, 7)).toBe(1); // after LoopIter
    expect(playheadToVisualInRange(events, 5, 2, 7)).toBe(2); // after Write (+ silent Alloc)
  });

  it("scrubs local visual back to a global playhead in range", () => {
    expect(visualToPlayheadInRange(events, 1, 2, 7)).toBe(3); // after LoopIter
    expect(visualToPlayheadInRange(events, 2, 2, 7)).toBe(5); // after Write at 4
  });
});
