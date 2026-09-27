import { describe, it, expect } from "vitest";
import { autoPack, canvasExtent } from "../src/layout";

describe("autoPack", () => {
  it("places first pane at gap origin", () => {
    expect(autoPack(100, 40, [], 400)).toEqual({ x: 8, y: 8 });
  });

  it("places next pane to the right on the same row", () => {
    expect(autoPack(80, 30, [{ x: 8, y: 8, w: 100, h: 40 }], 400)).toEqual({
      x: 116,
      y: 8,
    });
  });

  it("wraps to the next row when the row is full", () => {
    expect(
      autoPack(80, 30, [{ x: 8, y: 8, w: 100, h: 40 }], 180),
    ).toEqual({
      x: 8,
      y: 56,
    });
  });
});

describe("canvasExtent", () => {
  it("grows to fit panes", () => {
    const e = canvasExtent([{ x: 10, y: 20, w: 50, h: 30 }], 100, 100);
    expect(e.w).toBeGreaterThanOrEqual(10 + 50);
    expect(e.h).toBeGreaterThanOrEqual(20 + 30);
  });
});
