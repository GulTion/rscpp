import type { EventJson } from "@rscpp/timeline";

export type LoopSegment = {
  loop_id: number;
  startIndex: number;
  endIndex: number | null;
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
  const open = new Map<number, number>();
  const segs: LoopSegment[] = [];
  events.forEach((ev, i) => {
    if (ev.kind === "LoopIter") {
      const id = ev.loop_id as number;
      if (!open.has(id)) open.set(id, i);
    } else if (ev.kind === "LoopEnd") {
      const id = ev.loop_id as number;
      const start = open.get(id);
      if (start !== undefined) {
        segs.push({ loop_id: id, startIndex: start, endIndex: i });
        open.delete(id);
      }
    }
  });
  for (const [loop_id, startIndex] of open) {
    segs.push({ loop_id, startIndex, endIndex: null });
  }
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

  // Shallow first so deeper layers paint on top
  segs.sort((a, b) => a.depth - b.depth || a.startIndex - b.startIndex);
  return segs;
}

/** Stable color index from function name. */
export function colorIndexForName(name: string, paletteSize: number): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) | 0;
  return Math.abs(h) % Math.max(paletteSize, 1);
}
