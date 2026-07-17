import { describe, it, expect } from "vitest";
import { formatChip } from "../src/chips";

describe("formatChip", () => {
  it("formats ints and objects", () => {
    expect(formatChip({ kind: "Int", value: 5 })).toBe("⦃5⦄");
    expect(formatChip({ kind: "Object", value: 3 })).toBe("⦃#3⦄");
  });
});
