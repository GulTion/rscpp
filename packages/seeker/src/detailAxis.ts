import { isUiSilentAt } from "@rscpp/timeline";
import type { SilentEvent } from "@rscpp/timeline";

/** Visible events with indices in `[start, end]` inclusive. */
export function countVisibleInRange(
  events: SilentEvent[],
  start: number,
  end: number,
): number {
  const lo = Math.max(0, start);
  const hi = Math.min(end, events.length - 1);
  let n = 0;
  for (let i = lo; i <= hi; i++) {
    if (!isUiSilentAt(events, i)) n++;
  }
  return n;
}

/** Visual position within a segment for a global playhead. */
export function playheadToVisualInRange(
  events: SilentEvent[],
  playhead: number,
  start: number,
  end: number,
): number {
  const lim = Math.max(0, Math.min(Math.floor(playhead), events.length));
  const lo = Math.max(0, start);
  const hi = Math.min(end, events.length - 1);
  let v = 0;
  for (let i = lo; i < lim && i <= hi; i++) {
    if (!isUiSilentAt(events, i)) v++;
  }
  return v;
}

/** Map a local visual scrub value to a global playhead, clamped to the segment. */
export function visualToPlayheadInRange(
  events: SilentEvent[],
  visual: number,
  start: number,
  end: number,
): number {
  const target = Math.floor(visual + 1e-9);
  const lo = Math.max(0, start);
  const hi = Math.min(end, events.length - 1);
  if (target <= 0) return Math.min(lo + 1, hi + 1);
  let v = 0;
  for (let i = lo; i <= hi; i++) {
    if (isUiSilentAt(events, i)) continue;
    v++;
    if (v >= target) return i + 1;
  }
  return hi + 1;
}
