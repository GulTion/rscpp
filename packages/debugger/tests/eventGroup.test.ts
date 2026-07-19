import { describe, it, expect } from "vitest";
import { playheadEventGroup } from "../src/eventGroup";

const events = [
  { kind: "Step" },
  { kind: "VarAssign", name: "p" },
  { kind: "Write", slot: { kind: "Local", name: "p" } },
  { kind: "Write", slot: { kind: "MapEntry" } },
  { kind: "ContainerMod", op: "map_assign" },
];

describe("playheadEventGroup", () => {
  it("bundles leading silents with the non-silent step", () => {
    expect(playheadEventGroup(events, 0)).toEqual([]);
    expect(playheadEventGroup(events, 3).map((e) => e.kind)).toEqual([
      "Step",
      "VarAssign",
      "Write",
    ]);
    expect(playheadEventGroup(events, 5).map((e) => e.kind)).toEqual([
      "Write",
      "ContainerMod",
    ]);
  });
});
