export type Pos = { x: number; y: number };
export type Rect = Pos & { w: number; h: number };

const GAP = 8;

/** Place a new pane below existing rects (left-aligned). */
export function autoPack(w: number, h: number, occupied: Rect[]): Pos {
  if (occupied.length === 0) return { x: GAP, y: GAP };
  const maxY = Math.max(...occupied.map((r) => r.y + r.h));
  return { x: GAP, y: maxY + GAP };
}

export function applyPos(el: HTMLElement, pos: Pos): void {
  el.style.left = `${pos.x}px`;
  el.style.top = `${pos.y}px`;
}

/** Expand canvas so all panes (+ margin) remain reachable. */
export function canvasExtent(rects: Rect[], minW: number, minH: number): { w: number; h: number } {
  let w = minW;
  let h = minH;
  for (const r of rects) {
    w = Math.max(w, r.x + r.w + GAP * 2);
    h = Math.max(h, r.y + r.h + GAP * 2);
  }
  return { w, h };
}
