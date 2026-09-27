import { reconstruct, cloneSnapshot } from "./reconstruct.js";
import { isUiSilentAt, snapPlayheadIndex } from "./silent.js";
import type {
  EventJson,
  HighlightRange,
  HeapSnapshot,
  Timeline,
  TimelineEvent,
} from "./types.js";

function rangesFromEvent(ev: EventJson | undefined): HighlightRange[] {
  if (!ev?.span) return [];
  let kind = ev.kind;
  if (ev.kind === "Compare") {
    kind = ev.result === true ? "CompareTrue" : "CompareFalse";
  }
  return [{ start: ev.span.start, end: ev.span.end, kind }];
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

  /** Apply playhead without UI-silent snap (caller already chose the index). */
  function applyIndex(next: number): void {
    const clamped = Math.max(0, Math.min(Math.floor(next), events.length));
    if (clamped === index) {
      emit({ type: "highlight", ranges: currentHighlight() });
      return;
    }
    index = clamped;
    notify();
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
      applyIndex(snapPlayheadIndex(events, t));
    },
    step(delta: number) {
      const dir = delta === 0 ? 1 : Math.sign(delta);
      let left = Math.abs(delta) || 1;
      let i = index;
      while (left > 0) {
        const next = i + dir;
        if (next < 0 || next > events.length) break;
        i = next;
        while (
          i > 0 &&
          i < events.length &&
          isUiSilentAt(events, i - 1)
        ) {
          const n2 = i + dir;
          if (n2 < 0 || n2 > events.length) break;
          i = n2;
        }
        left--;
      }
      applyIndex(i);
    },
    play(opts?: { speed?: number }) {
      timeline.pause();
      const speed = Math.max(1, opts?.speed ?? 120);
      const batch = Math.max(1, Math.ceil(speed / 250));
      const ticksPerSec = speed / batch;
      const ms = Math.max(4, Math.floor(1000 / ticksPerSec));
      playTimer = setInterval(() => {
        if (index >= events.length) {
          timeline.pause();
          return;
        }
        timeline.step(batch);
        if (index >= events.length) timeline.pause();
      }, ms);
    },
    pause() {
      if (playTimer !== null) {
        clearInterval(playTimer);
        playTimer = null;
      }
    },
    get playing() {
      return playTimer !== null;
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
