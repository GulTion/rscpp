/** @vitest-environment jsdom */
import { describe, it, expect, afterEach } from "vitest";
import { createTimeline } from "@rscpp/timeline";
import { mountSeeker } from "../src/mount";

describe("segment start labels", () => {
  let root: HTMLElement;

  afterEach(() => {
    root?.remove();
  });

  it("shows static labels at fn/loop starts, no end logos", () => {
    root = document.createElement("div");
    document.body.appendChild(root);
    const events = [
      { kind: "FnEnter", name: "main", call_id: 0, parent_id: null },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "Step" },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "Step" },
      { kind: "LoopEnd", loop_id: 1, reason: "exhausted" },
      { kind: "FnExit", name: "main", call_id: 0, parent_id: null },
    ];
    const timeline = createTimeline({ events, source: "" });
    timeline.seek(events.length);
    const handle = mountSeeker(root, { timeline, source: "" });

    const fn = root.querySelector(
      '[data-testid="seeker-fn-start-0"]',
    ) as HTMLElement;
    const loop = root.querySelector(
      '[data-testid="seeker-loop-start-1"]',
    ) as HTMLElement;
    expect(fn).toBeTruthy();
    expect(loop).toBeTruthy();
    expect(fn.textContent).toContain("main");
    expect(loop.textContent).toContain("loop #1");
    // Labels sit on the segment band at its start.
    expect(fn.style.bottom).toBe("0px");
    expect(parseInt(loop.style.bottom, 10)).toBeGreaterThan(0);
    expect(root.querySelector('[data-testid="seeker-fn-end-0"]')).toBeNull();

    // One centered dot per LoopIter on the loop band
    const s1 = root.querySelector(
      '[data-testid="seeker-loop-step-1-1"]',
    ) as HTMLElement;
    const s2 = root.querySelector(
      '[data-testid="seeker-loop-step-1-2"]',
    ) as HTMLElement;
    expect(s1).toBeTruthy();
    expect(s2).toBeTruthy();
    expect(s1.style.borderRadius).toBe("50%");
    expect(s1.style.transform).toContain("translateX(-50%)");

    handle.destroy();
  });
});
