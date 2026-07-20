import { isUiSilentAt } from "@rscpp/timeline";
import type { SilentEvent } from "@rscpp/timeline";

/** Count of events that occupy space on the seekbar. */
export function countVisibleEvents(events: SilentEvent[]): number {
  let n = 0;
  for (let i = 0; i < events.length; i++) {
    if (!isUiSilentAt(events, i)) n++;
  }
  return n;
}

/** Visual position in `[0, visibleCount]` for a timeline playhead index. */
export function playheadToVisual(events: SilentEvent[], playhead: number): number {
  const lim = Math.max(0, Math.min(Math.floor(playhead), events.length));
  let v = 0;
  for (let i = 0; i < lim; i++) {
    if (!isUiSilentAt(events, i)) v++;
  }
  return v;
}

/** Visual position at the start of event `eventIndex` (array index). */
export function eventIndexToVisual(
  events: SilentEvent[],
  eventIndex: number,
): number {
  return playheadToVisual(events, eventIndex);
}

/** Map a visual scrub value back to a real playhead index. */
export function visualToPlayhead(events: SilentEvent[], visual: number): number {
  // floor so fractional scrub/play positions don't jump ahead of the thumb
  const target = Math.floor(visual + 1e-9);
  if (target <= 0) return 0;
  let v = 0;
  for (let i = 0; i < events.length; i++) {
    if (isUiSilentAt(events, i)) continue;
    v++;
    if (v >= target) return i + 1;
  }
  return events.length;
}
