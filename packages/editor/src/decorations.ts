import { StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView } from "@codemirror/view";
import type { HighlightRange } from "@rscpp/timeline";
import { buildByteIndexMap, spanBytesToJs } from "./spans.js";

const setHighlights = StateEffect.define<{
  ranges: HighlightRange[];
  source: string;
}>();

const KIND_COLOR: Record<string, string> = {
  Write: "#facc1588",
  VarCreate: "#38bdf899",
  VarAssign: "#facc1588",
  Alloc: "transparent",
  Dealloc: "transparent",
  RefBind: "transparent",
  ContainerMod: "#06b6d488",
  LoopIter: "#86efac99",
  LoopEnd: "#fca5a599",
  FnEnter: "#22c55e99",
  Call: "#a78bfa99",
  FnExit: "#fca5a599",
  CompareTrue: "#86efac99",
  CompareFalse: "#fca5a599",
  ScopeEnter: "transparent",
  ScopeExit: "transparent",
  Branch: "transparent",
  Step: "transparent",
  VarDestroy: "transparent",
  error: "#ef4444aa",
};

export function markColor(kind: string): string {
  return KIND_COLOR[kind] ?? "#64748b66";
}

export const highlightField = StateField.define({
  create() {
    return Decoration.none;
  },
  update(deco, tr) {
    for (const e of tr.effects) {
      if (e.is(setHighlights)) {
        const { ranges, source } = e.value;
        const map = buildByteIndexMap(source);
        const docLen = tr.state.doc.length;
        const built = ranges
          .filter((r) => r.end > r.start)
          .map((r) => {
            const { from, to } = spanBytesToJs(map, r.start, r.end);
            const a = Math.max(0, Math.min(from, docLen));
            const b = Math.max(a, Math.min(to, docLen));
            if (a >= b) return null;
            return Decoration.mark({
              attributes: {
                style: `background:${markColor(r.kind)};border-radius:2px;`,
                "data-hl-kind": r.kind,
              },
            }).range(a, b);
          })
          .filter((x): x is NonNullable<typeof x> => x !== null);
        return Decoration.set(built, true);
      }
    }
    return deco.map(tr.changes);
  },
  provide: (f) => EditorView.decorations.from(f),
});

export function applyHighlights(
  view: EditorView,
  ranges: HighlightRange[],
  source: string,
): void {
  const map = buildByteIndexMap(source);
  const first = ranges.find((r) => r.end > r.start);
  const effects: unknown[] = [setHighlights.of({ ranges, source })];
  if (first) {
    const { from } = spanBytesToJs(map, first.start, first.end);
    effects.push(
      EditorView.scrollIntoView(Math.min(from, view.state.doc.length), {
        y: "center",
      }),
    );
  }
  view.dispatch({ effects: effects as never });
}
