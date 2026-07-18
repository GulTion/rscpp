import type { EventJson, HeapSnapshot, ValueJson } from "@rscpp/timeline";

export type Cell = { i: number; j: number };

export type AccessHighlight = {
  /** 1D indices (array / table / stack / queue). */
  current: number[];
  trail: number[];
  /** 2D cells for matrix / nested vector. */
  currentCells: Cell[];
  trailCells: Cell[];
};

const TRAIL = 8;

function objId(v: unknown): number | null {
  if (!v || typeof v !== "object") return null;
  const o = v as { kind?: string; value?: unknown };
  if (o.kind === "Object" && typeof o.value === "number") return o.value;
  return null;
}

function intOf(v: unknown): number | null {
  if (!v || typeof v !== "object") return null;
  const o = v as ValueJson;
  if (o.kind === "Int" && typeof o.value === "number") return o.value;
  return null;
}

function rowMap(objId: number, heap: HeapSnapshot): Map<number, number> {
  const map = new Map<number, number>();
  const obj = heap.objects.get(objId);
  (obj?.elems ?? []).forEach((e, i) => {
    if (e.kind === "Object") map.set(e.value, i);
  });
  return map;
}

type Touch = { indices: number[]; cells: Cell[] };

function touchFromEvent(
  ev: EventJson,
  targetId: number,
  rows: Map<number, number>,
): Touch {
  const indices: number[] = [];
  const cells: Cell[] = [];

  if (ev.kind === "ContainerLookup" && ev.op === "index") {
    const cid = objId(ev.container);
    const key = intOf(ev.key);
    if (cid === targetId && key !== null) indices.push(key);
    if (cid !== null && rows.has(cid) && key !== null) {
      cells.push({ i: rows.get(cid)!, j: key });
      indices.push(rows.get(cid)!); // also mark the row in outer view if shown as list
    }
  }

  if (ev.kind === "Write") {
    const slot = ev.slot as { kind?: string; obj?: number; index?: number } | undefined;
    if (slot?.kind === "Index" && typeof slot.obj === "number" && typeof slot.index === "number") {
      if (slot.obj === targetId) indices.push(slot.index);
      if (rows.has(slot.obj)) {
        cells.push({ i: rows.get(slot.obj)!, j: slot.index });
        indices.push(rows.get(slot.obj)!);
      }
    }
  }

  // map / set keyed lookup
  if (ev.kind === "ContainerLookup" && (ev.op === "count" || ev.op === "index")) {
    const cid = objId(ev.container);
    if (cid === targetId) {
      const key = intOf(ev.key);
      if (key !== null) indices.push(key);
    }
  }

  return { indices: [...new Set(indices)], cells };
}

function cellKey(c: Cell): string {
  return `${c.i},${c.j}`;
}

/**
 * Element / cell accesses on `targetId` for events in `[0, timelineIndex)`.
 */
export function accessHighlight(
  events: EventJson[],
  timelineIndex: number,
  targetId: number,
  heap: HeapSnapshot,
): AccessHighlight {
  const rows = rowMap(targetId, heap);
  const t = Math.max(0, Math.min(timelineIndex, events.length));
  type Dated = Touch & { at: number };
  const seq: Dated[] = [];
  for (let i = 0; i < t; i++) {
    const touch = touchFromEvent(events[i], targetId, rows);
    if (touch.indices.length || touch.cells.length) seq.push({ ...touch, at: i });
  }
  if (seq.length === 0) {
    return { current: [], trail: [], currentCells: [], trailCells: [] };
  }

  const last = seq[seq.length - 1];
  let cur: Touch = { indices: [...last.indices], cells: [...last.cells] };
  for (let j = seq.length - 2; j >= 0; j--) {
    if (seq[j].at < last.at - 3) break;
    cur = {
      indices: [...new Set([...seq[j].indices, ...cur.indices])],
      cells: [...seq[j].cells, ...cur.cells],
    };
  }

  const curI = new Set(cur.indices);
  const curC = new Set(cur.cells.map(cellKey));

  const trail: number[] = [];
  const seenI = new Set<number>();
  for (let i = seq.length - 1; i >= 0 && trail.length < TRAIL; i--) {
    for (const idx of [...seq[i].indices].reverse()) {
      if (curI.has(idx) || seenI.has(idx)) continue;
      seenI.add(idx);
      trail.push(idx);
      if (trail.length >= TRAIL) break;
    }
  }
  trail.reverse();

  const trailCells: Cell[] = [];
  const seenC = new Set<string>();
  for (let i = seq.length - 1; i >= 0 && trailCells.length < TRAIL; i--) {
    for (const c of [...seq[i].cells].reverse()) {
      const k = cellKey(c);
      if (curC.has(k) || seenC.has(k)) continue;
      seenC.add(k);
      trailCells.push(c);
      if (trailCells.length >= TRAIL) break;
    }
  }
  trailCells.reverse();

  const currentCells: Cell[] = [];
  const ck = new Set<string>();
  for (const c of cur.cells) {
    const k = cellKey(c);
    if (ck.has(k)) continue;
    ck.add(k);
    currentCells.push(c);
  }

  return {
    current: [...curI],
    trail,
    currentCells,
    trailCells,
  };
}

export const EMPTY_ACCESS: AccessHighlight = {
  current: [],
  trail: [],
  currentCells: [],
  trailCells: [],
};

export function ensureAccessStyle(): void {
  if (typeof document === "undefined") return;
  if (document.getElementById("ds-viewer-style")) return;
  const style = document.createElement("style");
  style.id = "ds-viewer-style";
  style.textContent = `
.ds-flash { animation: dsflash 0.45s ease; background: #fef08a; }
@keyframes dsflash { from { background: #facc15; } to { background: transparent; } }
.ds-access-current {
  background: #fbbf24 !important;
  outline: 2px solid #d97706;
  animation: ds-access-pulse 0.55s ease-in-out infinite;
}
.ds-access-trail { background: #fde68a !important; outline: 1px solid #f59e0b; }
@keyframes ds-access-pulse {
  0%, 100% { filter: brightness(1); }
  50% { filter: brightness(1.12); }
}
`;
  document.head?.appendChild(style);
}
