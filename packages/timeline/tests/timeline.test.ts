import { describe, it, expect } from "vitest";
import { createTimeline } from "../src/createTimeline";
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
});
