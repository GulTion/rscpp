export type Pos = { x: number; y: number };
export type Rect = Pos & { w: number; h: number };

const GAP = 8;

/**
 * Place a new pane left→right on the current row; wrap to the next line when
 * it would exceed `rowWidth` (canvas width).
 */
export function autoPack(
  w: number,
  h: number,
  occupied: Rect[],
  rowWidth = 480,
): Pos {
  void h;
  if (occupied.length === 0) return { x: GAP, y: GAP };

  const maxW = Math.max(rowWidth, w + GAP * 2);
  const last = occupied[occupied.length - 1];
  const rowY = last.y;
  const rowItems = occupied.filter((r) => Math.abs(r.y - rowY) < 1);
  const rowRight = Math.max(...rowItems.map((r) => r.x + r.w));

  if (rowRight + GAP + w <= maxW) {
    return { x: rowRight + GAP, y: rowY };
  }

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
