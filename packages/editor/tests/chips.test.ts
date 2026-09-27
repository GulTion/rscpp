import { describe, it, expect } from "vitest";
import { EditorState } from "@codemirror/state";
import { formatChip, isSimpleChipValue } from "../src/chips";
import {
  lookupChipsFromEvents,
  localChipsFromFrames,
  frameBodySpans,
} from "../src/chipDecos";
import type { FrameState, HeapSnapshot, ValueJson } from "@rscpp/timeline";

describe("formatChip", () => {
  it("formats ints", () => {
    expect(formatChip({ kind: "Int", value: 5 })).toBe("5");
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
    expect(formatChip(ref, heap)).toBe("[2,7]");
    expect(isSimpleChipValue(ref, heap)).toBe(false);
    expect(isSimpleChipValue({ kind: "Int", value: 1 })).toBe(true);
  });

  it("does not chip closures", () => {
    const heap: HeapSnapshot = {
      objects: new Map([[5, { type_name: "closure" }]]),
      frames: [],
      openLoops: [],
    };
    expect(isSimpleChipValue({ kind: "Object", value: 5 }, heap)).toBe(false);
    expect(formatChip({ kind: "Object", value: 5 }, heap)).toBe("");
  });
});

describe("lookupChipsFromEvents", () => {
  it("places size result after nums.size()", () => {
    const source = "for (int i = 0; i < nums.size(); ++i) {}";
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
    expect(chips[0].text).toBe("4");
    expect(chips[0].from).toBe(end);
  });

  it("places index result after nums[i]", () => {
    const source = "int x = nums[i];";
    const start = source.indexOf("nums[i]");
    const end = start + "nums[i]".length;
    const events = [
      {
        kind: "ContainerLookup",
        op: "index",
        result: { kind: "Int", value: 8 },
        span: { start, end },
      },
    ];
    const chips = lookupChipsFromEvents(events, 1, source);
    expect(chips).toHaveLength(1);
    expect(chips[0].text).toBe("8");
    expect(chips[0].from).toBe(end);
  });
});

describe("localChipsFromFrames", () => {
  it("keeps nested same-name locals scoped to each function body", () => {
    const source = [
      "void A() { int i = 9; B(); }",
      "void B() { int i = 8; }",
    ].join("\n");
    const aStart = source.indexOf("void A()");
    const aEnd = source.indexOf("}") + 1;
    const bStart = source.indexOf("void B()");
    const bEnd = source.length;
    const events = [
      { kind: "FnEnter", call_id: 0, span: { start: aStart, end: aEnd } },
      { kind: "FnEnter", call_id: 1, span: { start: bStart, end: bEnd } },
    ];
    const frames: FrameState[] = [
      {
        call_id: 0,
        name: "A",
        parent_id: null,
        locals: new Map([["i", { kind: "Int", value: 9 }]]),
      },
      {
        call_id: 1,
        name: "B",
        parent_id: 0,
        locals: new Map([["i", { kind: "Int", value: 8 }]]),
      },
    ];
    const heap: HeapSnapshot = { objects: new Map(), frames, openLoops: [] };
    const state = EditorState.create({ doc: source });
    const bodyByCall = frameBodySpans(events, events.length);
    const chips = localChipsFromFrames(state, frames, heap, bodyByCall, source);

    const aI = source.indexOf("i = 9") + 1;
    const bI = source.indexOf("i = 8") + 1;
    expect(chips.find((c) => c.from === aI)?.text).toBe("9");
    expect(chips.find((c) => c.from === bI)?.text).toBe("8");
  });
});
