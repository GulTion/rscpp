import { describe, it, expect } from "vitest";
import { createTimeline } from "@rscpp/timeline";
import { mountDebugger } from "../src/mount";

describe("mountDebugger", () => {
  it("shows silent + non-silent batch for current step", () => {
    const events = [
      { kind: "Step", span: { start: 0, end: 1 } },
      { kind: "Alloc", span: { start: 1, end: 2 } },
      { kind: "Write", span: { start: 2, end: 3 }, name: "x" },
    ];
    const timeline = createTimeline({ events, source: "" });
    timeline.seek(3);

    const host = document.createElement("div");
    document.body.appendChild(host);
    const dbg = mountDebugger(host, { timeline });

    const btn = host.querySelector("[data-testid=debugger-toggle]") as HTMLButtonElement;
    btn.click();

    expect(host.querySelector("[data-testid=debugger-meta]")?.textContent).toContain(
      "Step · Alloc · Write",
    );
    const raw = host.querySelector("[data-testid=debugger-raw]")?.textContent ?? "";
    expect(raw).toContain('"kind": "Step"');
    expect(raw).toContain('"kind": "Write"');

    dbg.destroy();
    host.remove();
  });
});
