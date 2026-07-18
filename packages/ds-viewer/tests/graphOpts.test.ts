import { describe, it, expect } from "vitest";
import { normalizeEdges } from "../src/graphOpts";

describe("normalizeEdges", () => {
  it("undirected collapses both directions to one edge", () => {
    const out = normalizeEdges(
      [
        { from: 0, to: 1 },
        { from: 1, to: 0 },
        { from: 0, to: 2 },
      ],
      { direction: "undirected", multigraph: false },
    );
    expect(out).toHaveLength(2);
    expect(out.map((e) => `${e.from}-${e.to}`).sort()).toEqual(["0-1", "0-2"]);
  });

  it("directed keeps both directions", () => {
    const out = normalizeEdges(
      [
        { from: 0, to: 1 },
        { from: 1, to: 0 },
      ],
      { direction: "directed", multigraph: false },
    );
    expect(out).toHaveLength(2);
  });

  it("multigraph keeps parallel edges with indices", () => {
    const out = normalizeEdges(
      [
        { from: 0, to: 1 },
        { from: 0, to: 1 },
        { from: 1, to: 0 },
      ],
      { direction: "undirected", multigraph: true },
    );
    expect(out).toHaveLength(3);
    expect(out.filter((e) => e.from === 0 && e.to === 1).map((e) => e.index)).toEqual([
      0, 1, 2,
    ]);
    expect(out[0].count).toBe(3);
  });

  it("non-multigraph directed drops duplicate arcs", () => {
    const out = normalizeEdges(
      [
        { from: 0, to: 1 },
        { from: 0, to: 1 },
      ],
      { direction: "directed", multigraph: false },
    );
    expect(out).toHaveLength(1);
  });
});
