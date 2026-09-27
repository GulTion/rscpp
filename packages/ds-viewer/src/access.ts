import type { EventJson, HeapSnapshot, ValueJson } from "@rscpp/timeline";
import { formatVal } from "./diff.js";

export type Cell = { i: number; j: number };

export type StructuralOp = "push" | "pop" | "insert";

/**
 * Cell/index highlights:
 * - read → yellow (trail fades with age)
 * - write → green; value swap only on the current process write
 * - process → pulse
 */
export type AccessHighlight = {
  read: number[];
  write: number[];
  process: number[];
  readCells: Cell[];
  writeCells: Cell[];
  processCells: Cell[];
  /** Prior value for the *current* process write only (`"3"` or `"1,2"`). */
  writeOld: Map<string, string>;
  /** Keys that should run the write swap animation (process write only). */
  animateKeys: Set<string>;
  /** Read recency: 0 = newest in trail, higher = older (for opacity). */
  readAge: Map<string, number>;
  /** Process structural mutation (push/pop/map insert). */
  structural: { op: StructuralOp; index: number } | null;
  /** Process index lookup badge, e.g. `[i]=0`. */
  indexHint: { key: string; at: number | Cell } | null;
  /** Process swap of two indices. */
  swap: { a: number; b: number } | null;
};

const TRAIL = 10;

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

function rowMap(targetId: number, heap: HeapSnapshot): Map<number, number> {
  const map = new Map<number, number>();
  const obj = heap.objects.get(targetId);
  (obj?.elems ?? []).forEach((e, i) => {
    if (e.kind === "Object") map.set(e.value, i);
  });
  return map;
}

type Mode = "read" | "write";
type Touch = {
  indices: number[];
  cells: Cell[];
  mode: Mode;
  oldText?: string;
  structural?: StructuralOp;
  indexKey?: string;
  swap?: { a: number; b: number };
};

function slotIndex(
  slot: { kind?: string; obj?: number; index?: number } | undefined,
  targetId: number,
  rows: Map<number, number>,
): { indices: number[]; cells: Cell[] } {
  const indices: number[] = [];
  const cells: Cell[] = [];
  if (slot?.kind !== "Index" || typeof slot.obj !== "number" || typeof slot.index !== "number") {
    return { indices, cells };
  }
  if (slot.obj === targetId) indices.push(slot.index);
  if (rows.has(slot.obj)) {
    cells.push({ i: rows.get(slot.obj)!, j: slot.index });
    indices.push(rows.get(slot.obj)!);
  }
  return { indices, cells };
}

function touchFromEvent(
  ev: EventJson,
  targetId: number,
  rows: Map<number, number>,
  heap: HeapSnapshot,
): Touch {
  const indices: number[] = [];
  const cells: Cell[] = [];
  let mode: Mode = "read";
  let oldText: string | undefined;
  let structural: StructuralOp | undefined;
  let indexKey: string | undefined;
  let swap: { a: number; b: number } | undefined;
  const len = () => heap.objects.get(targetId)?.elems?.length ?? 0;

  if (ev.kind === "ContainerLookup") {
    const op = String(ev.op ?? "");
    if (["index", "count", "top", "front", "back", "size", "empty"].includes(op)) {
      const cid = objId(ev.container);
      const key = intOf(ev.key);
      if (cid === targetId && key !== null) {
        indices.push(key);
        if (op === "index") indexKey = String(key);
      }
      if (cid !== null && rows.has(cid) && key !== null) {
        cells.push({ i: rows.get(cid)!, j: key });
        indices.push(rows.get(cid)!);
        if (op === "index") indexKey = String(key);
      }
      mode = "read";
    }
  }

  if (ev.kind === "Write") {
    const slot = ev.slot as { kind?: string; obj?: number; index?: number } | undefined;
    const hit = slotIndex(slot, targetId, rows);
    if (hit.indices.length || hit.cells.length) {
      mode = "write";
      oldText = formatVal(ev.old as ValueJson | undefined);
      indices.push(...hit.indices);
      cells.push(...hit.cells);
    }
  }

  if (ev.kind === "ContainerMod") {
    const cid = objId(ev.container);
    const op = String(ev.op ?? "");
    let index = typeof ev.index === "number" ? ev.index : intOf(ev.key);
    if (cid === targetId || (cid !== null && rows.has(cid))) {
      mode = "write";
      oldText = formatVal(ev.old as ValueJson | undefined);
      if (
        op === "push_back" ||
        op === "emplace_back" ||
        op === "stack::push" ||
        op === "queue::push"
      ) {
        structural = "push";
        // Snapshot already includes the push → highlight last elem.
        if (cid === targetId && index === null) {
          const n = len();
          if (n > 0) index = n - 1;
        }
        if (cid === targetId && index !== null) indices.push(index);
      } else if (op === "pop_back" || op === "stack::pop" || op === "queue::pop") {
        structural = "pop";
        if (cid === targetId) {
          // After pop, removed index is previous length (== current length).
          indices.push(index ?? len());
        }
      } else if (op === "map_default_insert") {
        structural = "insert";
        if (cid === targetId && index !== null) indices.push(index);
      } else if (
        op === "index_assign" ||
        op === "map_assign" ||
        typeof ev.index === "number"
      ) {
        if (cid === targetId && index !== null) indices.push(index);
        if (cid !== null && rows.has(cid) && index !== null) {
          cells.push({ i: rows.get(cid)!, j: index });
          indices.push(rows.get(cid)!);
        }
      }
    }
  }

  if (ev.kind === "Swap") {
    const a = ev.a as { kind?: string; obj?: number; index?: number } | undefined;
    const b = ev.b as { kind?: string; obj?: number; index?: number } | undefined;
    const ha = slotIndex(a, targetId, rows);
    const hb = slotIndex(b, targetId, rows);
    if (ha.indices.length || hb.indices.length || ha.cells.length || hb.cells.length) {
      mode = "write";
      indices.push(...ha.indices, ...hb.indices);
      cells.push(...ha.cells, ...hb.cells);
      const ai = a?.obj === targetId && typeof a.index === "number" ? a.index : null;
      const bi = b?.obj === targetId && typeof b.index === "number" ? b.index : null;
      if (ai !== null && bi !== null) swap = { a: ai, b: bi };
      oldText = formatVal(ev.value_a as ValueJson | undefined);
    }
  }

  return {
    indices: [...new Set(indices)],
    cells,
    mode,
    oldText,
    structural,
    indexKey,
    swap,
  };
}

function cellKey(c: Cell): string {
  return `${c.i},${c.j}`;
}

function emptyAccess(): AccessHighlight {
  return {
    read: [],
    write: [],
    process: [],
    readCells: [],
    writeCells: [],
    processCells: [],
    writeOld: new Map(),
    animateKeys: new Set(),
    readAge: new Map(),
    structural: null,
    indexHint: null,
    swap: null,
  };
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
    const touch = touchFromEvent(events[i], targetId, rows, heap);
    if (
      touch.indices.length ||
      touch.cells.length ||
      touch.structural ||
      touch.swap
    ) {
      seq.push({ ...touch, at: i });
    }
  }

  if (seq.length === 0) return emptyAccess();

  const processIdx = new Set<number>();
  const processCells: Cell[] = [];
  const processCk = new Set<string>();
  const readIdx = new Set<number>();
  const writeIdx = new Set<number>();
  const readCells: Cell[] = [];
  const writeCells: Cell[] = [];
  const readCk = new Set<string>();
  const writeCk = new Set<string>();
  const writeOld = new Map<string, string>();
  const animateKeys = new Set<string>();
  const readAge = new Map<string, number>();

  const lastAt = seq[seq.length - 1].at;
  const processAt = t - 1;
  let structural: AccessHighlight["structural"] = null;
  let indexHint: AccessHighlight["indexHint"] = null;
  let swap: AccessHighlight["swap"] = null;

  for (let i = seq.length - 1; i >= 0; i--) {
    const s = seq[i];
    if (s.at < lastAt - TRAIL && s.at !== processAt) break;
    const isProcess = s.at === processAt;
    const age = lastAt - s.at;

    for (const idx of s.indices) {
      if (isProcess) processIdx.add(idx);
      if (s.mode === "write") {
        writeIdx.add(idx);
        if (isProcess && s.oldText !== undefined && s.oldText !== "") {
          writeOld.set(String(idx), s.oldText);
          animateKeys.add(String(idx));
        }
      } else {
        readIdx.add(idx);
        const k = String(idx);
        if (!readAge.has(k) || age < readAge.get(k)!) readAge.set(k, age);
      }
    }
    for (const c of s.cells) {
      const k = cellKey(c);
      if (isProcess && !processCk.has(k)) {
        processCk.add(k);
        processCells.push(c);
      }
      if (s.mode === "write") {
        if (!writeCk.has(k)) {
          writeCk.add(k);
          writeCells.push(c);
        }
        if (isProcess && s.oldText !== undefined && s.oldText !== "") {
          writeOld.set(k, s.oldText);
          animateKeys.add(k);
        }
      } else if (!writeCk.has(k)) {
        if (!readCk.has(k)) {
          readCk.add(k);
          readCells.push(c);
        }
        if (!readAge.has(k) || age < readAge.get(k)!) readAge.set(k, age);
      }
    }

    if (isProcess) {
      if (s.structural && s.indices[0] !== undefined) {
        structural = { op: s.structural, index: s.indices[0] };
        if (s.structural === "insert" || s.structural === "push") {
          animateKeys.add(String(s.indices[0]));
        }
      }
      if (s.indexKey !== undefined) {
        const at = s.cells[0] ?? s.indices[0];
        if (at !== undefined) indexHint = { key: s.indexKey, at };
      }
      if (s.swap) swap = s.swap;
    }
  }

  for (const i of writeIdx) readIdx.delete(i);
  for (const c of writeCells) {
    readCk.delete(cellKey(c));
    readAge.delete(cellKey(c));
  }
  for (const i of writeIdx) readAge.delete(String(i));
  const readCellsFiltered = readCells.filter((c) => !writeCk.has(cellKey(c)));

  return {
    read: [...readIdx].filter((i) => !writeIdx.has(i)),
    write: [...writeIdx],
    process: [...processIdx],
    readCells: readCellsFiltered,
    writeCells,
    processCells,
    writeOld,
    animateKeys,
    readAge,
    structural,
    indexHint,
    swap,
  };
}

export const EMPTY_ACCESS: AccessHighlight = emptyAccess();

export function markIndexAccess(
  el: HTMLElement,
  index: number,
  access: AccessHighlight,
): void {
  const isWrite = access.write.includes(index);
  const isRead = access.read.includes(index);
  const isProcess = access.process.includes(index);
  if (isWrite) el.classList.add("ds-access-write");
  else if (isRead || isProcess) el.classList.add("ds-access-read");
  if (isProcess) el.classList.add("ds-access-process");
  if (access.swap && (access.swap.a === index || access.swap.b === index)) {
    el.classList.add("ds-access-swap");
  }
  if (access.structural?.index === index) {
    el.classList.add(`ds-struct-${access.structural.op}`);
  }
  const age = access.readAge.get(String(index));
  if (age !== undefined && !isWrite) {
    el.style.opacity = String(Math.max(0.35, 1 - age * 0.08));
  }
}

export function markCellAccess(
  el: HTMLElement,
  cell: Cell,
  access: AccessHighlight,
): void {
  const k = cellKey(cell);
  const isWrite = access.writeCells.some((c) => cellKey(c) === k);
  const isRead = access.readCells.some((c) => cellKey(c) === k);
  const isProcess = access.processCells.some((c) => cellKey(c) === k);
  if (isWrite) el.classList.add("ds-access-write");
  else if (isRead || isProcess) el.classList.add("ds-access-read");
  if (isProcess) el.classList.add("ds-access-process");
  const age = access.readAge.get(k);
  if (age !== undefined && !isWrite) {
    el.style.opacity = String(Math.max(0.35, 1 - age * 0.08));
  }
}

function animKeyFor(indexOrCell: number | Cell): string {
  return typeof indexOrCell === "number"
    ? String(indexOrCell)
    : cellKey(indexOrCell);
}

/** True only for the current process write (avoids replaying old→new every paint). */
export function shouldAnimateWrite(
  access: AccessHighlight,
  indexOrCell: number | Cell,
): boolean {
  return access.animateKeys.has(animKeyFor(indexOrCell));
}

export function attachIndexHint(
  el: HTMLElement,
  access: AccessHighlight,
  indexOrCell: number | Cell,
): void {
  const hint = access.indexHint;
  if (!hint) return;
  const match =
    typeof hint.at === "number"
      ? typeof indexOrCell === "number" && hint.at === indexOrCell
      : typeof indexOrCell !== "number" &&
        hint.at.i === indexOrCell.i &&
        hint.at.j === indexOrCell.j;
  if (!match) return;
  const badge = document.createElement("span");
  badge.className = "ds-index-hint";
  badge.textContent = `[${hint.key}]`;
  el.appendChild(badge);
}

/** Fill cell text; optional bottom→top swap from old→new on process write. */
export function fillAccessCell(
  el: HTMLElement,
  nextText: string,
  oldText: string | undefined,
  animateWrite: boolean,
): void {
  el.textContent = "";
  if (animateWrite && oldText !== undefined && oldText !== "" && oldText !== nextText) {
    const clip = document.createElement("div");
    clip.className = "ds-value-swap";
    const inner = document.createElement("div");
    inner.className = "ds-value-swap-inner";
    const oldEl = document.createElement("div");
    oldEl.className = "ds-value-swap-line";
    oldEl.textContent = oldText;
    const newEl = document.createElement("div");
    newEl.className = "ds-value-swap-line";
    newEl.textContent = nextText;
    inner.append(oldEl, newEl);
    clip.appendChild(inner);
    el.appendChild(clip);
    return;
  }
  if (animateWrite && (!oldText || oldText === "")) {
    const slide = document.createElement("div");
    slide.className = "ds-value-enter";
    slide.textContent = nextText;
    el.appendChild(slide);
    return;
  }
  el.textContent = nextText;
}

export function ensureAccessStyle(): void {
  if (typeof document === "undefined") return;
  let style = document.getElementById("ds-viewer-style") as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement("style");
    style.id = "ds-viewer-style";
    document.head?.appendChild(style);
  }
  style.textContent = `
.ds-flash { animation: dsflash 0.45s ease; background: #fef08a; }
@keyframes dsflash { from { background: #facc15; } to { background: transparent; } }
.ds-access-read {
  background: #fde047 !important;
  outline: 2px solid #ca8a04;
}
.ds-access-write {
  background: #86efac !important;
  outline: 2px solid #16a34a;
}
.ds-access-process {
  animation: ds-access-pulse 0.55s ease-in-out infinite;
}
@keyframes ds-access-pulse {
  0%, 100% { filter: brightness(1); }
  50% { filter: brightness(1.1); }
}
.ds-access-swap {
  animation: ds-swap-nudge 0.45s ease;
}
@keyframes ds-swap-nudge {
  0% { transform: translateX(0); }
  40% { transform: translateX(6px); }
  100% { transform: translateX(0); }
}
.ds-struct-push, .ds-struct-insert {
  animation: ds-cell-in 0.4s ease;
}
.ds-struct-pop {
  animation: ds-cell-out 0.35s ease forwards;
}
@keyframes ds-cell-in {
  from { transform: translateY(8px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
@keyframes ds-cell-out {
  from { transform: translateY(0); opacity: 1; }
  to { transform: translateY(-8px); opacity: 0.2; }
}
.ds-value-swap {
  height: 1.25em;
  overflow: hidden;
  line-height: 1.25em;
  position: relative;
}
.ds-value-swap-inner {
  display: flex;
  flex-direction: column;
  animation: ds-swap-up 0.45s ease forwards;
}
.ds-value-swap-line {
  height: 1.25em;
  line-height: 1.25em;
  text-align: center;
  flex: 0 0 1.25em;
}
@keyframes ds-swap-up {
  from { transform: translateY(0); }
  to { transform: translateY(-1.25em); }
}
.ds-value-enter {
  animation: ds-cell-in 0.4s ease;
}
.ds-index-hint {
  display: block;
  font: 9px ui-monospace, monospace;
  color: #854d0e;
  line-height: 1;
  margin-top: 1px;
}
`;
}
