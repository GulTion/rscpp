import { describe, it, expect } from "vitest";
import {
  countVisibleEvents,
  eventIndexToVisual,
  playheadToVisual,
  visualToPlayhead,
} from "../src/visibleAxis";

const events = [
  { kind: "Step" },
  { kind: "Alloc" },
  { kind: "VarAssign", name: "p" },
  { kind: "Write", slot: { kind: "Local", name: "p" } },
  { kind: "Write", slot: { kind: "MapEntry", key: { kind: "Int", value: 2 } } },
  { kind: "ContainerMod", op: "map_assign" },
];

describe("visibleAxis", () => {
  it("counts only non-silent events", () => {
    // Local Write + ContainerMod
    expect(countVisibleEvents(events)).toBe(2);
  });

  it("maps playhead past silents without growing visual axis", () => {
    expect(playheadToVisual(events, 0)).toBe(0);
    expect(playheadToVisual(events, 3)).toBe(0); // Step+Alloc+VarAssign
    expect(playheadToVisual(events, 4)).toBe(1); // Local Write
    expect(playheadToVisual(events, 5)).toBe(1); // MapEntry Write silent
    expect(playheadToVisual(events, 6)).toBe(2); // ContainerMod
  });

  it("round-trips scrub positions to non-silent playheads", () => {
    expect(visualToPlayhead(events, 0)).toBe(0);
    expect(visualToPlayhead(events, 1)).toBe(4); // after Local Write
    expect(visualToPlayhead(events, 2)).toBe(6); // after ContainerMod
    expect(eventIndexToVisual(events, 3)).toBe(0);
    expect(eventIndexToVisual(events, 5)).toBe(1);
  });
});
