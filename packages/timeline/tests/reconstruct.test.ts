import { describe, it, expect } from "vitest";
import { reconstruct } from "../src/reconstruct";
import fixture from "../src/fixtures/vector_push.json";
import twoSum from "../src/fixtures/two_sum.json";

describe("reconstruct", () => {
  it("t=0 is empty", () => {
    const s = reconstruct(fixture.events, 0);
    expect(s.objects.size).toBe(0);
  });

  it("after Alloc+push sees elems", () => {
    const s = reconstruct(fixture.events, fixture.events.length);
    const v = s.objects.get(0);
    expect(v?.type_name).toBe("vector");
    expect(v?.elems?.map((e) => (e as { value: number }).value)).toEqual([1, 2]);
  });

  it("map Write+ContainerMod does not duplicate keys", () => {
    // map id 2 is deallocated before end of fixture — stop while it is live
    const deallocAt = twoSum.events.findIndex(
      (e) => e.kind === "Dealloc" && e.id === 2,
    );
    const s = reconstruct(twoSum.events, deallocAt);
    const m = s.objects.get(2);
    expect(m?.type_name).toBe("map");
    expect(m?.entries).toEqual([
      { key: { kind: "Int", value: 2 }, value: { kind: "Int", value: 0 } },
    ]);
  });

  it("does not treat closure Alloc as empty array", () => {
    const s = reconstruct(
      [{ kind: "Alloc", id: 5, type_name: "closure", size: 0, elems: [] }],
      1,
    );
    const c = s.objects.get(5);
    expect(c?.type_name).toBe("closure");
    expect(c?.elems).toBeUndefined();
  });

  it("applies Swap to index slots", () => {
    const events = [
      {
        kind: "Alloc",
        id: 0,
        type_name: "vector",
        elems: [
          { kind: "Int", value: 1 },
          { kind: "Int", value: 9 },
        ],
      },
      {
        kind: "Swap",
        a: { kind: "Index", obj: 0, index: 0 },
        b: { kind: "Index", obj: 0, index: 1 },
        value_a: { kind: "Int", value: 1 },
        value_b: { kind: "Int", value: 9 },
      },
    ];
    const s = reconstruct(events, 2);
    expect(s.objects.get(0)?.elems?.map((e) => (e as { value: number }).value)).toEqual([
      9, 1,
    ]);
  });
});
