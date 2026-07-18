/** @vitest-environment jsdom */
import { describe, it, expect, beforeEach } from "vitest";
import { createTimeline } from "@rscpp/timeline";
import { mountDsViewer } from "../src/mount";

// jsdom lacks PointerEvent / capture — stub enough for drag.
if (typeof PointerEvent === "undefined") {
  // @ts-expect-error test polyfill
  globalThis.PointerEvent = class PointerEvent extends MouseEvent {
    pointerId: number;
    constructor(type: string, init: MouseEventInit & { pointerId?: number } = {}) {
      super(type, init);
      this.pointerId = init.pointerId ?? 1;
    }
  };
}
HTMLElement.prototype.setPointerCapture = function () {};
HTMLElement.prototype.releasePointerCapture = function () {};

const events = [
  { kind: "FnEnter", call_id: 1, name: "main", parent_id: null },
  {
    kind: "Alloc",
    id: 1,
    type_name: "vector",
    elems: [{ kind: "Int", value: 1 }],
  },
  {
    kind: "VarCreate",
    name: "a",
    value: { kind: "Object", value: 1 },
  },
  {
    kind: "Alloc",
    id: 2,
    type_name: "vector",
    elems: [{ kind: "Int", value: 2 }],
  },
  {
    kind: "VarCreate",
    name: "b",
    value: { kind: "Object", value: 2 },
  },
  { kind: "Dealloc", id: 1 },
  {
    kind: "Alloc",
    id: 1,
    type_name: "vector",
    elems: [{ kind: "Int", value: 9 }],
  },
  {
    kind: "VarCreate",
    name: "a",
    value: { kind: "Object", value: 1 },
  },
];

function drag(handle: HTMLElement, dx: number, dy: number): void {
  handle.dispatchEvent(
    new PointerEvent("pointerdown", {
      button: 0,
      clientX: 0,
      clientY: 0,
      bubbles: true,
      pointerId: 1,
    }),
  );
  handle.dispatchEvent(
    new PointerEvent("pointermove", {
      clientX: dx,
      clientY: dy,
      bubbles: true,
      pointerId: 1,
    }),
  );
  handle.dispatchEvent(
    new PointerEvent("pointerup", {
      clientX: dx,
      clientY: dy,
      bubbles: true,
      pointerId: 1,
    }),
  );
}

describe("ds canvas layout", () => {
  let root: HTMLElement;

  beforeEach(() => {
    root = document.createElement("div");
    document.body.appendChild(root);
    // give canvas a size for packing
    root.style.width = "400px";
    root.style.height = "300px";
  });

  it("keeps position by id across dealloc and re-alloc", () => {
    const timeline = createTimeline({ events });
    timeline.seek(5); // a + b live
    mountDsViewer(root, { timeline, objId: null, mode: "all" });

    expect(root.querySelector('[data-testid="ds-pane-title-1"]')?.textContent).toBe(
      "a",
    );

    const handle = root.querySelector(
      '[data-testid="ds-pane-handle-1"]',
    ) as HTMLElement;
    const pane = root.querySelector('[data-testid="ds-pane-1"]') as HTMLElement;
    expect(pane).toBeTruthy();
    const ox = parseInt(pane.style.left, 10);
    const oy = parseInt(pane.style.top, 10);
    drag(handle, 120, 80);
    expect(pane.style.left).toBe(`${ox + 120}px`);
    expect(pane.style.top).toBe(`${oy + 80}px`);
    const savedLeft = pane.style.left;
    const savedTop = pane.style.top;

    timeline.seek(6); // id 1 gone (dealloc); local may still point — object missing
    expect(root.querySelector('[data-testid="ds-pane-1"]')).toBeNull();

    timeline.seek(8); // id 1 + VarCreate a back
    const again = root.querySelector('[data-testid="ds-pane-1"]') as HTMLElement;
    expect(again).toBeTruthy();
    expect(again.style.left).toBe(savedLeft);
    expect(again.style.top).toBe(savedTop);
  });

  it("Reset layout clears saved positions", () => {
    const timeline = createTimeline({ events });
    timeline.seek(5);
    mountDsViewer(root, { timeline, objId: null, mode: "all" });
    const handle = root.querySelector(
      '[data-testid="ds-pane-handle-1"]',
    ) as HTMLElement;
    const before = root.querySelector('[data-testid="ds-pane-1"]') as HTMLElement;
    drag(handle, 200, 100);
    expect(parseInt(before.style.left, 10)).toBeGreaterThan(8);

    (root.querySelector('[data-testid="ds-reset-layout"]') as HTMLButtonElement).click();
    const after = root.querySelector('[data-testid="ds-pane-1"]') as HTMLElement;
    // Function Tree packs first; variables pack below it
    expect(parseInt(after.style.left, 10)).toBe(8);
    expect(parseInt(after.style.top, 10)).toBeGreaterThanOrEqual(8);
    expect(root.querySelector('[data-testid="ds-pane-fn-tree"]')).toBeTruthy();
  });

  it("auto-packs a second object below the first", () => {
    const timeline = createTimeline({ events });
    timeline.seek(5);
    mountDsViewer(root, { timeline, objId: null, mode: "all" });
    const a = root.querySelector('[data-testid="ds-pane-1"]') as HTMLElement;
    const b = root.querySelector('[data-testid="ds-pane-2"]') as HTMLElement;
    expect(parseInt(b.style.top, 10)).toBeGreaterThan(parseInt(a.style.top, 10));
    expect(root.querySelector('[data-testid="ds-fn-tree"]')).toBeTruthy();
  });
});
