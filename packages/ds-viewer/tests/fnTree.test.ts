import { describe, it, expect } from "vitest";
import {
  argNamesAfterEnter,
  buildFnTree,
  formatFnLabel,
  argUnchangedFromParent,
} from "../src/fnTree";
import type { EventJson } from "@rscpp/timeline";

const events: EventJson[] = [
  {
    kind: "FnEnter",
    call_id: 0,
    parent_id: null,
    name: "Solution::countComponents",
    args: [
      { kind: "Int", value: 6 },
      { kind: "Object", value: 6 },
    ],
  },
  { kind: "VarCreate", name: "n", value: { kind: "Int", value: 6 } },
  {
    kind: "VarCreate",
    name: "edges",
    value: { kind: "Object", value: 6 },
  },
  { kind: "ScopeEnter" },
  {
    kind: "FnEnter",
    call_id: 1,
    parent_id: 0,
    name: "Solution::dfs",
    args: [
      { kind: "Int", value: 0 },
      { kind: "Object", value: 6 },
      { kind: "Object", value: 8 },
    ],
  },
  { kind: "VarCreate", name: "u", value: { kind: "Int", value: 0 } },
  {
    kind: "VarCreate",
    name: "adj",
    value: { kind: "Ref", value: { kind: "Heap", value: 6 } },
  },
  {
    kind: "VarCreate",
    name: "vis",
    value: { kind: "Ref", value: { kind: "Heap", value: 8 } },
  },
  { kind: "ScopeEnter" },
  {
    kind: "FnEnter",
    call_id: 2,
    parent_id: 1,
    name: "Solution::dfs",
    args: [
      { kind: "Int", value: 1 },
      { kind: "Object", value: 6 },
      { kind: "Object", value: 8 },
    ],
  },
  { kind: "VarCreate", name: "u", value: { kind: "Int", value: 1 } },
  {
    kind: "VarCreate",
    name: "adj",
    value: { kind: "Ref", value: { kind: "Heap", value: 6 } },
  },
  {
    kind: "VarCreate",
    name: "vis",
    value: { kind: "Ref", value: { kind: "Heap", value: 8 } },
  },
  { kind: "ScopeEnter" },
  { kind: "FnExit", call_id: 2, parent_id: 1, name: "Solution::dfs" },
  { kind: "FnExit", call_id: 1, parent_id: 0, name: "Solution::dfs" },
];

describe("argNamesAfterEnter", () => {
  it("reads VarCreate names before ScopeEnter", () => {
    expect(argNamesAfterEnter(events, 4)).toEqual(["u", "adj", "vis"]);
  });
});

describe("buildFnTree", () => {
  it("builds nested dfs with arg names and active stack", () => {
    // After both dfs enters (index 14 = before first exit)
    const roots = buildFnTree(events, 14);
    expect(roots).toHaveLength(1);
    expect(roots[0].name).toContain("countComponents");
    const dfs0 = roots[0].children[0];
    expect(dfs0.args.map((a) => a.name)).toEqual(["u", "adj", "vis"]);
    expect(formatFnLabel(dfs0)).toContain("u=0");
    expect(formatFnLabel(dfs0)).toContain("vis=#8");
    const dfs1 = dfs0.children[0];
    expect(dfs1.args.find((a) => a.name === "u")?.value).toEqual({
      kind: "Int",
      value: 1,
    });
    expect(argUnchangedFromParent(dfs0, dfs1.args.find((a) => a.name === "vis")!)).toBe(
      true,
    );
    expect(argUnchangedFromParent(dfs0, dfs1.args.find((a) => a.name === "u")!)).toBe(
      false,
    );
    expect(dfs1.active).toBe(true);
  });

  it("marks exited frames inactive after FnExit", () => {
    const roots = buildFnTree(events, events.length);
    const dfs0 = roots[0].children[0];
    expect(dfs0.active).toBe(false);
    expect(dfs0.children[0].active).toBe(false);
  });
});
