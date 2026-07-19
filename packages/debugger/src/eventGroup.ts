import { isUiSilentAt } from "@rscpp/timeline";
import type { SilentEvent } from "@rscpp/timeline";

/**
 * Events consumed by one visible step ending at `playhead`:
 * leading silent run + the non-silent event (e.g. S1 S2 E1).
 */
export function playheadEventGroup<T extends SilentEvent>(
  events: T[],
  playhead: number,
): T[] {
  if (playhead <= 0) return [];
  const end = Math.min(Math.floor(playhead), events.length);
  let start = end - 1;
  while (start > 0 && isUiSilentAt(events, start - 1)) start--;
  return events.slice(start, end);
}
