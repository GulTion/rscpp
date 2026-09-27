import type { EditorState } from "@codemirror/state";
import { Decoration } from "@codemirror/view";
import type {
  EventJson,
  FrameState,
  HeapSnapshot,
  Span,
  Timeline,
  ValueJson,
} from "@rscpp/timeline";
import { formatChip, chipTitle, isSimpleChipValue } from "./chips.js";
import { buildByteIndexMap, spanBytesToJs } from "./spans.js";
import { WidgetType } from "@codemirror/view";

export class ChipWidget extends WidgetType {
  constructor(
    readonly text: string,
    readonly title: string,
    readonly variant: "local" | "lookup" = "local",
  ) {
    super();
  }
  eq(other: ChipWidget) {
    return (
      this.text === other.text &&
      this.title === other.title &&
      this.variant === other.variant
    );
  }
  toDOM() {
    const span = document.createElement("span");
    span.textContent = this.text;
    span.title = this.title;
    span.dataset.testid = "editor-chip";
    span.dataset.chipVariant = this.variant;
    // local = teal; lookup (index/size/…) = amber
    span.style.cssText =
      this.variant === "local"
        ? "font-size:0.75em;color:var(--chip-local-fg, #0f766e);background:var(--chip-local-bg, #ccfbf1);margin-left:2px;border-radius:3px;padding:0 2px;"
        : "font-size:0.75em;color:var(--chip-lookup-fg, #b45309);background:var(--chip-lookup-bg, #fef3c7);margin-left:2px;border-radius:3px;padding:0 2px;";
    return span;
  }
  ignoreEvent() {
    return true;
  }
}

type ChipAt = { from: number; text: string; title: string; variant: "local" | "lookup" };

/** FnEnter span per call_id (function body / def range in source bytes). */
export function frameBodySpans(
  events: EventJson[],
  index: number,
): Map<number, Span> {
  const out = new Map<number, Span>();
  const n = Math.max(0, Math.min(index, events.length));
  for (let i = 0; i < n; i++) {
    const ev = events[i];
    if (ev.kind !== "FnEnter") continue;
    const callId = ev.call_id;
    const span = ev.span;
    if (typeof callId !== "number" || !span || span.end <= span.start) continue;
    out.set(callId, span);
  }
  return out;
}

/** Latest ContainerLookup results keyed by span, for chips like `nums.size() 4` / `nums[i] 8`. */
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
    if (!["size", "empty", "count", "top", "front", "back", "index"].includes(op)) continue;
    const span = ev.span;
    const result = ev.result as ValueJson | undefined;
    if (!span || span.end <= span.start || !result) continue;
    const key = `${span.start}:${span.end}:${op}`;
    latest.set(key, { start: span.start, end: span.end, result, op });
  }
  const map = buildByteIndexMap(source);
  const out: ChipAt[] = [];
  for (const { start, end, result } of latest.values()) {
    if (!isSimpleChipValue(result)) continue;
    const { to } = spanBytesToJs(map, start, end);
    out.push({
      from: to,
      text: formatChip(result),
      title: chipTitle(result),
      variant: "lookup",
    });
  }
  return out;
}

/**
 * Place local chips only inside each frame's FnEnter span, so nested functions
 * with the same local name don't overwrite each other globally.
 */
export function localChipsFromFrames(
  state: EditorState,
  frames: FrameState[],
  heap: HeapSnapshot,
  bodyByCall: Map<number, Span>,
  source: string,
): ChipAt[] {
  if (frames.length === 0) return [];
  const text = state.doc.toString();
  const byteMap = buildByteIndexMap(source);
  // Outer → inner; same doc offset keeps the innermost frame's value.
  const byFrom = new Map<number, ChipAt>();

  for (const frame of frames) {
    const body = bodyByCall.get(frame.call_id);
    if (!body) continue;
    const { from: bodyFrom, to: bodyTo } = spanBytesToJs(
      byteMap,
      body.start,
      body.end,
    );
    for (const [name, value] of frame.locals) {
      if (name.length === 0) continue;
      if (!isSimpleChipValue(value, heap)) continue;
      const re = new RegExp(
        `\\b${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`,
        "g",
      );
      let m: RegExpExecArray | null;
      while ((m = re.exec(text))) {
        const at = m.index;
        if (at < bodyFrom || at >= bodyTo) continue;
        const after = text.slice(at + m[0].length);
        if (/^\s*\.\s*(size|empty|count|top|front|back)\s*\(/.test(after)) continue;
        const from = at + m[0].length;
        byFrom.set(from, {
          from,
          text: formatChip(value, heap),
          title: chipTitle(value, heap),
          variant: "local",
        });
      }
    }
  }
  return [...byFrom.values()];
}

export function buildChipDecos(state: EditorState, timeline: Timeline) {
  const snap = timeline.snapshot();
  const source = timeline.source || state.doc.toString();
  const bodyByCall = frameBodySpans(timeline.events, timeline.index);
  const chips = [
    ...localChipsFromFrames(
      state,
      snap.frames,
      snap,
      bodyByCall,
      source,
    ),
    ...lookupChipsFromEvents(timeline.events, timeline.index, source),
  ];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const ranges: any[] = chips
    .filter((c) => c.from >= 0 && c.from <= state.doc.length)
    .map((c) =>
      Decoration.widget({
        widget: new ChipWidget(c.text, c.title, c.variant),
        side: 1,
      }).range(c.from),
    );
  return Decoration.set(ranges, true);
}
