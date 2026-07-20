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

export type ActiveSegment = {
  kind: "loop" | "call";
  /** Inclusive event-array start index. */
  startIndex: number;
  /** Inclusive event-array end index. */
  endIndex: number;
  label: string;
  depth: number;
  loop_id?: number;
  call_id?: number;
  name?: string;
};

function segmentContainsPlayhead(
  playhead: number,
  startIndex: number,
  endIndex: number | null,
  eventCount: number,
): boolean {
  const end = endIndex ?? eventCount - 1;
  if (end < startIndex || eventCount === 0) return false;
  // Playhead t means events[0..t) applied; inside after start event, through end event.
  return playhead > startIndex && playhead <= end + 1;
}

type Cand = {
  kind: "loop" | "call";
  startIndex: number;
  endIndex: number;
  /** How many other call/loop spans fully contain this one (higher = more nested). */
  nest: number;
  label: string;
  depth: number;
  loop_id?: number;
  call_id?: number;
  name?: string;
};

function shortName(name: string): string {
  const i = name.lastIndexOf("::");
  return i >= 0 ? name.slice(i + 2) : name;
}

/**
 * Innermost segment containing the playhead, by containment nesting.
 * A function nested inside an outer while beats that while; a loop inside
 * the function beats the function. (Previously any loop beat every call,
 * so detail seekbar never showed nested functions like dfs.)
 */
export function currentSegment(
  events: EventJson[],
  playhead: number,
): ActiveSegment | null {
  const p = Math.max(0, Math.min(Math.floor(playhead), events.length));
  const n = events.length;
  const calls = buildCallSegments(events);
  const loops = buildLoopSegments(events);

  type Span = { startIndex: number; endIndex: number };
  const spans: Span[] = [
    ...calls.map((c) => ({
      startIndex: c.startIndex,
      endIndex: c.endIndex ?? n - 1,
    })),
    ...loops.map((l) => ({
      startIndex: l.startIndex,
      endIndex: l.endIndex ?? n - 1,
    })),
  ];

  const nestOf = (start: number, end: number): number => {
    let nest = 0;
    for (const o of spans) {
      if (o.startIndex === start && o.endIndex === end) continue;
      if (o.startIndex <= start && o.endIndex >= end && (o.startIndex < start || o.endIndex > end)) {
        nest++;
      }
    }
    return nest;
  };

  const cands: Cand[] = [];
  for (const s of loops) {
    if (!segmentContainsPlayhead(p, s.startIndex, s.endIndex, n)) continue;
    const endIndex = s.endIndex ?? n - 1;
    cands.push({
      kind: "loop",
      startIndex: s.startIndex,
      endIndex,
      nest: nestOf(s.startIndex, endIndex),
      label: `loop #${s.loop_id}`,
      depth: s.depth,
      loop_id: s.loop_id,
    });
  }
  for (const s of calls) {
    if (!segmentContainsPlayhead(p, s.startIndex, s.endIndex, n)) continue;
    const endIndex = s.endIndex ?? n - 1;
    cands.push({
      kind: "call",
      startIndex: s.startIndex,
      endIndex,
      nest: nestOf(s.startIndex, endIndex),
      label: shortName(s.name),
      depth: s.depth,
      call_id: s.call_id,
      name: s.name,
    });
  }
  if (cands.length === 0) return null;

  cands.sort(
    (a, b) =>
      b.nest - a.nest ||
      // tighter span wins ties
      a.endIndex - a.startIndex - (b.endIndex - b.startIndex),
  );
  const best = cands[0];
  return {
    kind: best.kind,
    startIndex: best.startIndex,
    endIndex: best.endIndex,
    label: best.label,
    depth: best.depth,
    loop_id: best.loop_id,
    call_id: best.call_id,
    name: best.name,
  };
}

/** Stable color index from function name. */
export function colorIndexForName(name: string, paletteSize: number): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) | 0;
  return Math.abs(h) % Math.max(paletteSize, 1);
}
