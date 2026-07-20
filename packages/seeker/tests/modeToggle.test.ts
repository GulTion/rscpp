/** @vitest-environment jsdom */
import { describe, it, expect, afterEach } from "vitest";
import { createTimeline } from "@rscpp/timeline";
import { mountSeeker } from "../src/mount";

describe("seeker mode toggle", () => {
  let root: HTMLElement;

  afterEach(() => {
    root?.remove();
  });

  it("hides advanced stack/detail and shows a simple track", () => {
    root = document.createElement("div");
    document.body.appendChild(root);
    const events = [
      { kind: "FnEnter", name: "main", call_id: 0, parent_id: null },
      { kind: "LoopIter", loop_id: 1 },
      { kind: "Step" },
      { kind: "LoopEnd", loop_id: 1, reason: "exhausted" },
      { kind: "FnExit", name: "main", call_id: 0, parent_id: null },
    ];
    const timeline = createTimeline({ events, source: "" });
    timeline.seek(3);
    const handle = mountSeeker(root, { timeline, source: "" });

    const btn = root.querySelector(
      '[data-testid="seeker-mode-toggle"]',
    ) as HTMLButtonElement;
    expect(btn.textContent).toMatch(/Hide advanced/i);
    expect(
      (root.querySelector('[data-testid="seeker-stack"]') as HTMLElement).style
        .display,
    ).not.toBe("none");

    btn.click();
    expect(btn.textContent).toMatch(/Show advanced/i);
    expect(
      (root.querySelector('[data-testid="seeker-stack"]') as HTMLElement).style
        .display,
    ).toBe("none");
    expect(
      (root.querySelector('[data-testid="seeker-detail"]') as HTMLElement).style
        .display,
    ).toBe("none");
    expect(
      (root.querySelector('[data-testid="seeker-track"]') as HTMLElement).style
        .height,
    ).toBe("18px");
    expect(root.querySelector('[data-seg]')).toBeNull();

    btn.click();
    expect(btn.textContent).toMatch(/Hide advanced/i);
    expect(
      (root.querySelector('[data-testid="seeker-stack"]') as HTMLElement).style
        .display,
    ).not.toBe("none");

    handle.destroy();
  });
});
