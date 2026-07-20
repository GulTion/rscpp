/** @vitest-environment jsdom */
import { describe, it, expect, afterEach } from "vitest";
import { createTimeline } from "@rscpp/timeline";
import { mountSeeker } from "../src/mount";
import { LAYER_H } from "../src/stackLayout";

describe("stacked seekbar mount", () => {
  let root: HTMLElement;

  afterEach(() => {
    root?.remove();
  });

  it("renders nested call slabs with increasing stack bottoms", () => {
    root = document.createElement("div");
    document.body.appendChild(root);
    const events = [
      { kind: "FnEnter", name: "main", call_id: 0, parent_id: null },
      { kind: "Step" },
      { kind: "FnEnter", name: "fn", call_id: 1, parent_id: 0 },
      { kind: "Step" },
      { kind: "FnExit", name: "fn", call_id: 1, parent_id: 0 },
      { kind: "FnExit", name: "main", call_id: 0, parent_id: null },
    ];
    const timeline = createTimeline({ events, source: "" });
    timeline.seek(events.length);
    const handle = mountSeeker(root, { timeline, source: "" });

    const stack = root.querySelector('[data-testid="seeker-stack"]');
    expect(stack).toBeTruthy();

    const main = root.querySelector('[data-seg="fn-0"]') as HTMLElement;
    const fn = root.querySelector('[data-seg="fn-1"]') as HTMLElement;
    expect(main).toBeTruthy();
    expect(fn).toBeTruthy();
    expect(Number(main.dataset.stackDepth)).toBe(0);
    expect(Number(fn.dataset.stackDepth)).toBe(1);

    const mainBottom = parseInt(main.style.bottom, 10);
    const fnBottom = parseInt(fn.style.bottom, 10);
    expect(fnBottom).toBe(mainBottom + LAYER_H);
    expect(fnBottom).toBeGreaterThan(mainBottom);

    handle.destroy();
  });
});
