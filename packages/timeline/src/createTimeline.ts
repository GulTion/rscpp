import { reconstruct, cloneSnapshot } from "./reconstruct.js";
import type {
  EventJson,
  HighlightRange,
  HeapSnapshot,
  Timeline,
  TimelineEvent,
} from "./types.js";

function rangesFromEvent(ev: EventJson | undefined): HighlightRange[] {
  if (!ev?.span) return [];
  return [{ start: ev.span.start, end: ev.span.end, kind: ev.kind }];
}

export function createTimeline(opts: {
  events: EventJson[];
  source?: string;
}): Timeline {
  const events = opts.events;
  const source = opts.source ?? "";
  let index = 0;
  let snapshotCache: HeapSnapshot = reconstruct(events, 0);
  let hover: HighlightRange[] | null = null;
  let playTimer: ReturnType<typeof setInterval> | null = null;
  const listeners = new Set<(ev: TimelineEvent) => void>();

  function emit(ev: TimelineEvent): void {
    for (const l of listeners) l(ev);
  }

  function currentHighlight(): HighlightRange[] {
    if (hover) return hover;
    if (index === 0) return [];
    return rangesFromEvent(events[index - 1]);
  }

  function notify(): void {
    snapshotCache = reconstruct(events, index);
    emit({ type: "seek", index });
    emit({ type: "tick", index, snapshot: cloneSnapshot(snapshotCache) });
    emit({ type: "highlight", ranges: currentHighlight() });
  }

  const timeline: Timeline = {
    get length() {
      return events.length;
    },
    get index() {
      return index;
    },
    get source() {
      return source;
    },
    get events() {
      return events;
    },
    seek(t: number) {
      const next = Math.max(0, Math.min(Math.floor(t), events.length));
      if (next === index) {
        emit({ type: "highlight", ranges: currentHighlight() });
        return;
      }
      index = next;
      notify();
    },
    step(delta: number) {
      timeline.seek(index + delta);
    },
    play(opts?: { speed?: number }) {
      timeline.pause();
      const speed = Math.max(1, opts?.speed ?? 120);
      // Batch steps when targeting high event rates so the UI isn't starved.
      const batch = Math.max(1, Math.ceil(speed / 250));
      const ticksPerSec = speed / batch;
      const ms = Math.max(4, Math.floor(1000 / ticksPerSec));
      playTimer = setInterval(() => {
        if (index >= events.length) {
          timeline.pause();
          return;
        }
        timeline.seek(Math.min(index + batch, events.length));
      }, ms);
    },
    pause() {
      if (playTimer !== null) {
        clearInterval(playTimer);
        playTimer = null;
      }
    },
    snapshot() {
      return cloneSnapshot(snapshotCache);
    },
    highlight() {
      return currentHighlight();
    },
    setHoverHighlight(ranges: HighlightRange[] | null) {
      hover = ranges;
      emit({ type: "highlight", ranges: currentHighlight() });
    },
    subscribe(listener: (ev: TimelineEvent) => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };

  return timeline;
}
