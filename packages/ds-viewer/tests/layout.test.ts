import { describe, it, expect } from "vitest";
import { autoPack, canvasExtent } from "../src/layout";

describe("autoPack", () => {
  it("places first pane at gap origin", () => {
    expect(autoPack(100, 40, [])).toEqual({ x: 8, y: 8 });
  });

  it("stacks below occupied rects", () => {
    expect(autoPack(80, 30, [{ x: 8, y: 8, w: 100, h: 40 }])).toEqual({
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
