import type { EditorState } from "@codemirror/state";
import { Decoration } from "@codemirror/view";
import type { EventJson, HeapSnapshot, Timeline, ValueJson } from "@rscpp/timeline";
import { formatChip, chipTitle } from "./chips.js";
import { buildByteIndexMap, spanBytesToJs } from "./spans.js";
import { WidgetType } from "@codemirror/view";

export class ChipWidget extends WidgetType {
  constructor(
    readonly text: string,
    readonly title: string,
  ) {
    super();
  }
  eq(other: ChipWidget) {
    return this.text === other.text && this.title === other.title;
  }
  toDOM() {
    const span = document.createElement("span");
    span.textContent = this.text;
    span.title = this.title;
    span.style.cssText =
      "font-size:0.75em;color:#0f766e;background:#ccfbf1;margin-left:2px;border-radius:3px;padding:0 2px;";
    span.dataset.testid = "editor-chip";
    return span;
  }
  ignoreEvent() {
    return true;
  }
}

type ChipAt = { from: number; text: string; title: string };

/** Latest ContainerLookup results (size/empty/top/…) keyed by span, for chips like `nums.size()⦃4⦄`. */
export function lookupChipsFromEvents(
  events: EventJson[],
  index: number,
  source: string,
): ChipAt[] {
  const latest = new Map<string, { start: number; end: number; result: ValueJson; op: string }>();
  const n = Math.max(0, Math.min(index, events.length));
  for (let i = 0; i < n; i++) {
    const ev = events[i];
    if (ev.kind !== "ContainerLookup") continue;
    const op = String(ev.op ?? "");
    if (!["size", "empty", "count", "top", "front", "back"].includes(op)) continue;
    const span = ev.span;
    const result = ev.result as ValueJson | undefined;
    if (!span || span.end <= span.start || !result) continue;
    const key = `${span.start}:${span.end}:${op}`;
    latest.set(key, { start: span.start, end: span.end, result, op });
  }
  const map = buildByteIndexMap(source);
  const out: ChipAt[] = [];
  for (const { start, end, result } of latest.values()) {
    const { to } = spanBytesToJs(map, start, end);
    out.push({
      from: to,
      text: formatChip(result),
      title: chipTitle(result),
    });
  }
  return out;
}

export function localChipsFromSnapshot(
  state: EditorState,
  locals: Map<string, ValueJson>,
  heap: HeapSnapshot,
): ChipAt[] {
  if (locals.size === 0) return [];
  const text = state.doc.toString();
  const out: ChipAt[] = [];
  for (const [name, value] of locals) {
    if (name.length === 0) continue;
    const re = new RegExp(`\\b${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`, "g");
    let m: RegExpExecArray | null;
    while ((m = re.exec(text))) {
      // Skip if this match is the receiver of `.size()` / `.empty()` etc. —
      // those get a lookup chip after the full call expression instead.
      const after = text.slice(m.index + m[0].length);
      if (/^\s*\.\s*(size|empty|count|top|front|back)\s*\(/.test(after)) continue;
      out.push({
        from: m.index + m[0].length,
        text: formatChip(value, heap),
        title: chipTitle(value, heap),
      });
    }
  }
  return out;
}

export function buildChipDecos(state: EditorState, timeline: Timeline) {
  const snap = timeline.snapshot();
  const locals = snap.frames.at(-1)?.locals ?? new Map<string, ValueJson>();
  const source = timeline.source || state.doc.toString();
  const chips = [
    ...localChipsFromSnapshot(state, locals, snap),
    ...lookupChipsFromEvents(timeline.events, timeline.index, source),
  ];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const ranges: any[] = chips
    .filter((c) => c.from >= 0 && c.from <= state.doc.length)
    .map((c) =>
      Decoration.widget({
        widget: new ChipWidget(c.text, c.title),
        side: 1,
      }).range(c.from),
    );
  return Decoration.set(ranges, true);
}
