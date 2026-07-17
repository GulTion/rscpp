import { describe, it, expect } from "vitest";
import { buildLoopSegments } from "../src/segments";

describe("buildLoopSegments", () => {
  it("pairs loop segments", () => {
    const events = [
      { kind: "Step" },
      { kind: "Step" },
      { kind: "Step" },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "Step" },
      { kind: "LoopEnd", loop_id: 1, reason: "exhausted" },
    ];
    const segs = buildLoopSegments(events);
    expect(segs[0]).toMatchObject({ loop_id: 1, startIndex: 3, endIndex: 5 });
  });
});
