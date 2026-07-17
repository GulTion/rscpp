import { StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView } from "@codemirror/view";
import type { HighlightRange } from "@rscpp/timeline";

const setHighlights = StateEffect.define<HighlightRange[]>();

const KIND_COLOR: Record<string, string> = {
  Write: "#f59e0b55",
  Alloc: "#22c55e55",
  ContainerMod: "#06b6d455",
  LoopIter: "#3b82f655",
  LoopEnd: "#3b82f633",
  FnEnter: "#a855f755",
  FnExit: "#a855f733",
  Step: "#94a3b833",
  error: "#ef444488",
};

function markColor(kind: string): string {
  return KIND_COLOR[kind] ?? "#64748b44";
}

export const highlightField = StateField.define({
  create() {
    return Decoration.none;
  },
  update(deco, tr) {
    for (const e of tr.effects) {
      if (e.is(setHighlights)) {
        const ranges = e.value
          .filter((r) => r.start < r.end)
          .map((r) =>
            Decoration.mark({
              attributes: {
                style: `background:${markColor(r.kind)}`,
                "data-hl-kind": r.kind,
              },
            }).range(r.start, Math.min(r.end, tr.state.doc.length)),
          );
        return Decoration.set(ranges, true);
      }
    }
    return deco.map(tr.changes);
  },
  provide: (f) => EditorView.decorations.from(f),
});

export function applyHighlights(view: EditorView, ranges: HighlightRange[]): void {
  view.dispatch({ effects: setHighlights.of(ranges) });
}
