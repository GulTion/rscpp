import { describe, it, expect } from "vitest";
import { eventIndexForLine, lineByteRange } from "../src/lineSeek";

describe("lineSeek", () => {
  it("maps line to byte range and finds overlapping event", () => {
    const source = "int a;\nint b;\nint c;\n";
    const r0 = lineByteRange(source, 0)!;
    const r1 = lineByteRange(source, 1)!;
    expect(source.slice(r0.start, r0.end)).toMatch(/^int a;/);
    expect(source.slice(r1.start, r1.end)).toMatch(/^int b;/);

    const events = [
      { kind: "Step", span: { start: r1.start, end: r1.start + 5 } },
    ];
    expect(eventIndexForLine(events, source, 1)).toBe(0);
    expect(eventIndexForLine(events, source, 0)).toBeNull();
  });
});
