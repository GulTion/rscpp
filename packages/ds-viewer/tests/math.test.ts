/** @vitest-environment jsdom */
import { describe, it, expect } from "vitest";
import { toTex, setMathContent } from "../src/math";

describe("toTex", () => {
  it("keeps bare numbers", () => {
    expect(toTex("42")).toBe("42");
    expect(toTex("-3.5")).toBe("-3.5");
  });

  it("formats object ids and text", () => {
    expect(toTex("#8")).toBe("\\#8");
    expect(toTex("nullptr")).toBe("\\mathrm{nullptr}");
    expect(toTex("hello")).toBe("\\text{hello}");
  });
});

describe("setMathContent", () => {
  it("writes katex html for a number", () => {
    const el = document.createElement("span");
    setMathContent(el, "7");
    expect(el.classList.contains("ds-math")).toBe(true);
    expect(el.querySelector(".katex")).toBeTruthy();
    expect(el.textContent).toContain("7");
  });
});
