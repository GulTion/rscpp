import type { EventJson } from "@rscpp/timeline";

export type LoopSegment = {
  loop_id: number;
  startIndex: number;
  endIndex: number | null;
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
