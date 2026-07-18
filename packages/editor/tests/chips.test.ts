import { describe, it, expect } from "vitest";
import { formatChip, isSimpleChipValue } from "../src/chips";
import { lookupChipsFromEvents } from "../src/chipDecos";
import type { HeapSnapshot, ValueJson } from "@rscpp/timeline";

describe("formatChip", () => {
  it("formats ints", () => {
    expect(formatChip({ kind: "Int", value: 5 })).toBe("⦃5⦄");
  });

  it("resolves Ref to vector elems (for title/debug)", () => {
    const heap: HeapSnapshot = {
      objects: new Map([
        [
          0,
          {
            type_name: "vector",
            elems: [
              { kind: "Int", value: 2 },
              { kind: "Int", value: 7 },
            ] as ValueJson[],
          },
        ],
      ]),
      frames: [],
      openLoops: [],
    };
    const ref = {
      kind: "Ref" as const,
      value: { kind: "Heap", value: 0 },
    };
    expect(formatChip(ref, heap)).toBe("⦃[2,7]⦄");
    expect(isSimpleChipValue(ref, heap)).toBe(false);
    expect(isSimpleChipValue({ kind: "Int", value: 1 })).toBe(true);
  });
});

describe("lookupChipsFromEvents", () => {
  it("places size result after nums.size()", () => {
    const source = "for (int i = 0; i < nums.size(); ++i) {}";
    // Find byte offsets of nums.size()
    const start = source.indexOf("nums.size()");
    const end = start + "nums.size()".length;
    const events = [
      {
        kind: "ContainerLookup",
        op: "size",
        result: { kind: "Int", value: 4 },
        span: { start, end },
      },
    ];
    const chips = lookupChipsFromEvents(events, 1, source);
    expect(chips).toHaveLength(1);
    expect(chips[0].text).toBe("⦃4⦄");
    expect(chips[0].from).toBe(end);
  });
});
