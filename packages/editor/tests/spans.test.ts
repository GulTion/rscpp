import { describe, it, expect } from "vitest";
import { buildByteIndexMap, byteToJs, jsToByte, spanBytesToJs } from "../src/spans";

describe("byte↔js spans", () => {
  it("identity on ascii", () => {
    const s = "hello\nworld";
    const map = buildByteIndexMap(s);
    expect(byteToJs(map, 0)).toBe(0);
    expect(byteToJs(map, 5)).toBe(5);
    expect(jsToByte(map, 6)).toBe(6);
  });

  it("maps past multibyte em dash", () => {
    const s = "a—b"; // U+2014 → 3 UTF-8 bytes
    const map = buildByteIndexMap(s);
    expect(new TextEncoder().encode(s).length).toBe(5);
    expect(byteToJs(map, 0)).toBe(0);
    expect(byteToJs(map, 1)).toBe(1);
    expect(byteToJs(map, 4)).toBe(2);
    expect(spanBytesToJs(map, 1, 4)).toEqual({ from: 1, to: 2 });
    expect(s.slice(1, 2)).toBe("—");
  });

  it("aligns with two_sum-style comment dash", () => {
    const s = "// LeetCode 1. Two Sum — class\nint x;";
    const map = buildByteIndexMap(s);
    const dash = s.indexOf("—");
    const byteDash = jsToByte(map, dash);
    expect(byteToJs(map, byteDash)).toBe(dash);
    const after = s.indexOf("\nint");
    expect(byteToJs(map, jsToByte(map, after))).toBe(after);
  });
});
