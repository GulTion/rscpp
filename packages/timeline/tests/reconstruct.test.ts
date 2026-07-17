import { describe, it, expect } from "vitest";
import { reconstruct } from "../src/reconstruct";
import fixture from "../src/fixtures/vector_push.json";

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
});
