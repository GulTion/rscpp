import type { EventJson } from "@rscpp/timeline";

export type LoopSegment = {
  loop_id: number;
  startIndex: number;
  endIndex: number | null;
  /** Nesting depth among open loops (0 = outermost). */
  depth: number;
};

export type CallSegment = {
  call_id: number;
  parent_id: number | null;
  name: string;
  startIndex: number;
  endIndex: number | null;
  /** 0 = outermost (e.g. main / root method). */
  depth: number;
};

/** Pair LoopIter start with LoopEnd for each loop_id instance. */
export function buildLoopSegments(events: EventJson[]): LoopSegment[] {
  type Open = { startIndex: number; depth: number };
  const open = new Map<number, Open>();
  const segs: LoopSegment[] = [];
  events.forEach((ev, i) => {
    if (ev.kind === "LoopIter") {
      const id = ev.loop_id as number;
      if (!open.has(id)) {
        open.set(id, { startIndex: i, depth: open.size });
      }
    } else if (ev.kind === "LoopEnd") {
      const id = ev.loop_id as number;
      const o = open.get(id);
      if (o !== undefined) {
        segs.push({
          loop_id: id,
          startIndex: o.startIndex,
          endIndex: i,
          depth: o.depth,
        });
        open.delete(id);
      }
    }
  });
  for (const [loop_id, o] of open) {
    segs.push({
      loop_id,
      startIndex: o.startIndex,
      endIndex: null,
      depth: o.depth,
    });
  }
  segs.sort((a, b) => a.depth - b.depth || a.startIndex - b.startIndex);
  return segs;
}

/**
 * Pair FnEnter/FnExit into nested call segments.
 * Depth follows parent_id chain (root depth 0).
 */
export function buildCallSegments(events: EventJson[]): CallSegment[] {
  type Open = {
    call_id: number;
    parent_id: number | null;
    name: string;
    startIndex: number;
  };
  const open = new Map<number, Open>();
  const depthOf = new Map<number, number>();
  const segs: CallSegment[] = [];

  events.forEach((ev, i) => {
    if (ev.kind === "FnEnter") {
      const call_id = ev.call_id as number;
      const parent_id =
        ev.parent_id === undefined || ev.parent_id === null
          ? null
          : (ev.parent_id as number);
      const name = String(ev.name ?? "?");
      const depth =
        parent_id === null ? 0 : (depthOf.get(parent_id) ?? 0) + 1;
      depthOf.set(call_id, depth);
      open.set(call_id, { call_id, parent_id, name, startIndex: i });
    } else if (ev.kind === "FnExit") {
      const call_id = ev.call_id as number;
      const o = open.get(call_id);
      if (o) {
        segs.push({
          call_id: o.call_id,
          parent_id: o.parent_id,
          name: o.name,
          startIndex: o.startIndex,
          endIndex: i,
          depth: depthOf.get(call_id) ?? 0,
        });
        open.delete(call_id);
      }
    }
  });

  for (const o of open.values()) {
    segs.push({
      call_id: o.call_id,
      parent_id: o.parent_id,
      name: o.name,
      startIndex: o.startIndex,
      endIndex: null,
      depth: depthOf.get(o.call_id) ?? 0,
    });
  }

  segs.sort((a, b) => a.depth - b.depth || a.startIndex - b.startIndex);
  return segs;
}

/** Stable color index from function name. */
export function colorIndexForName(name: string, paletteSize: number): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) | 0;
  return Math.abs(h) % Math.max(paletteSize, 1);
}
