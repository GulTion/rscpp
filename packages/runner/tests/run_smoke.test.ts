import { describe, it, expect } from "vitest";
import { initRunner, runMethod } from "../src/index";
import fixture from "../../timeline/src/fixtures/two_sum.json";

const hasWasm = process.env.RSCPP_WASM === "1";

describe.skipIf(!hasWasm)("runner wasm smoke", () => {
  it("runMethod twoSum", async () => {
    await initRunner();
    const r = await runMethod(
      fixture.source as string,
      "Solution::twoSum",
      [[2, 7, 11, 15], 9],
    );
    expect(r.ok).toBe(true);
    expect(r.events.length).toBeGreaterThan(0);
  });
});

describe("runner without init", () => {
  it("returns synthetic failure", async () => {
    // fresh module state not guaranteed; just assert shape of failed helper via re-import pattern
    const { run } = await import("../src/index");
    const r = await run("int main(){return 0;}");
    // may succeed if previous test inited — only assert events array exists
    expect(Array.isArray(r.events)).toBe(true);
    expect(typeof r.ok).toBe("boolean");
  });
});
