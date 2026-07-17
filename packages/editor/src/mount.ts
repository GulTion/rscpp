import {
  EditorView,
  Decoration,
  WidgetType,
  ViewPlugin,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { cpp } from "@codemirror/lang-cpp";
import type { Timeline, ValueJson } from "@rscpp/timeline";
import type { RunResult } from "@rscpp/runner";
import { applyHighlights, highlightField } from "./decorations.js";
import { formatChip } from "./chips.js";
import { buildByteIndexMap, jsToByte } from "./spans.js";

export type MountHandle = {
  update(props: Partial<EditorProps>): void;
  destroy(): void;
};

export type EditorProfile = "leetcode" | "codeforces";

export type EditorProps = {
  timeline: Timeline;
  source: string;
  profile: EditorProfile;
  method?: string;
  argsJson?: string;
  expectedJson?: string;
  onRun?: (result: RunResult) => void;
  runMain?: (source: string) => Promise<RunResult>;
  runMethod?: (
    source: string,
    method: string,
    args: unknown[],
  ) => Promise<RunResult>;
};

class ChipWidget extends WidgetType {
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

function buildChipDecos(state: EditorState, locals: Map<string, ValueJson>) {
  if (locals.size === 0) return Decoration.none;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const ranges: any[] = [];
  const text = state.doc.toString();
  for (const [name, value] of locals) {
    const re = new RegExp(`\\b${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`, "g");
    let m: RegExpExecArray | null;
    while ((m = re.exec(text))) {
      const from = m.index + m[0].length;
      ranges.push(
        Decoration.widget({
          widget: new ChipWidget(formatChip(value), JSON.stringify(value)),
          side: 1,
        }).range(from),
      );
    }
  }
  return Decoration.set(ranges, true);
}

export function mountEditor(el: HTMLElement, props: EditorProps): MountHandle {
  let timeline = props.timeline;
  let profile = props.profile;
  let method = props.method ?? "Solution::twoSum";
  let argsJson = props.argsJson ?? "[[2,7,11,15],9]";
  let expectedJson = props.expectedJson ?? "";
  let onRun = props.onRun;
  let runMain = props.runMain;
  let runMethod = props.runMethod;

  el.dataset.testid = "editor-root";
  el.innerHTML = "";

  const toolbar = document.createElement("div");
  toolbar.style.cssText = "display:flex;flex-wrap:wrap;gap:6px;margin-bottom:6px;align-items:center;";

  const profileSel = document.createElement("select");
  profileSel.dataset.testid = "editor-profile";
  for (const p of ["leetcode", "codeforces"] as const) {
    const o = document.createElement("option");
    o.value = p;
    o.textContent = p;
    if (p === profile) o.selected = true;
    profileSel.appendChild(o);
  }

  const methodInput = document.createElement("input");
  methodInput.dataset.testid = "editor-method";
  methodInput.value = method;
  methodInput.placeholder = "Solution::method";
  methodInput.style.width = "160px";

  const argsInput = document.createElement("input");
  argsInput.dataset.testid = "editor-args";
  argsInput.value = argsJson;
  argsInput.style.cssText = "flex:1;min-width:120px";

  const runBtn = document.createElement("button");
  runBtn.textContent = "Run";
  runBtn.dataset.testid = "editor-run";

  const errorBox = document.createElement("div");
  errorBox.dataset.testid = "editor-error";
  errorBox.style.cssText = "color:#b91c1c;font:12px monospace;min-height:1.2em;";

  const ioStub = document.createElement("textarea");
  ioStub.dataset.testid = "editor-stdin-stub";
  ioStub.disabled = true;
  ioStub.placeholder = "stdin (coming soon)";
  ioStub.style.cssText = "width:100%;height:48px;display:none;margin-bottom:6px;";

  const cmHost = document.createElement("div");
  cmHost.style.cssText = "height:calc(100% - 80px);min-height:200px;border:1px solid #e5e7eb;";

  toolbar.append(profileSel, methodInput, argsInput, runBtn);
  el.append(toolbar, ioStub, errorBox, cmHost);

  function syncProfileUi(): void {
    const lc = profile === "leetcode";
    methodInput.style.display = lc ? "" : "none";
    argsInput.style.display = lc ? "" : "none";
    ioStub.style.display = lc ? "none" : "block";
  }
  syncProfileUi();

  const chipPlugin = ViewPlugin.fromClass(
    class {
      decorations = Decoration.none;
      constructor(view: EditorView) {
        this.rebuild(view);
      }
      rebuild(view: EditorView) {
        const locals =
          timeline.snapshot().frames.at(-1)?.locals ?? new Map<string, ValueJson>();
        this.decorations = buildChipDecos(view.state, locals);
      }
      update(u: { view: EditorView; docChanged: boolean }) {
        if (u.docChanged) this.rebuild(u.view);
      }
    },
    { decorations: (v) => v.decorations },
  );

  let view = new EditorView({
    parent: cmHost,
    state: EditorState.create({
      doc: props.source,
      extensions: [
        lineNumbers(),
        history(),
        cpp(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        highlightField,
        chipPlugin,
        EditorView.theme({
          "&": { height: "100%", fontSize: "13px" },
          ".cm-scroller": { overflow: "auto" },
        }),
        EditorView.domEventHandlers({
          click: (_e, v) => {
            const pos = v.state.selection.main.head;
            const doc = v.state.doc.toString();
            const map = buildByteIndexMap(doc);
            const bytePos = jsToByte(map, pos);
            const events = timeline.events;
            let best = -1;
            let bestDist = Infinity;
            for (let i = 0; i < events.length; i++) {
              const sp = events[i].span;
              if (!sp || sp.end <= sp.start) continue;
              if (bytePos >= sp.start && bytePos < sp.end) {
                best = i;
                break;
              }
              const mid = (sp.start + sp.end) / 2;
              const d = Math.abs(mid - bytePos);
              if (d < bestDist) {
                bestDist = d;
                best = i;
              }
            }
            if (best >= 0) timeline.seek(best + 1);
            return false;
          },
        }),
      ],
    }),
  });

  function paintHighlights(): void {
    // Spans are UTF-8 bytes into the event source (= timeline.source).
    applyHighlights(view, timeline.highlight(), timeline.source || view.state.doc.toString());
  }

  let unsub = timeline.subscribe((ev) => {
    if (ev.type === "highlight" || ev.type === "tick" || ev.type === "seek") {
      paintHighlights();
    }
    if (ev.type === "tick") {
      const plugin = view.plugin(chipPlugin);
      plugin?.rebuild(view);
      view.dispatch({});
    }
  });
  paintHighlights();

  profileSel.addEventListener("change", () => {
    profile = profileSel.value as EditorProfile;
    syncProfileUi();
  });

  runBtn.addEventListener("click", async () => {
    errorBox.textContent = "";
    const source = view.state.doc.toString();
    let result: RunResult;
    if (profile === "leetcode") {
      if (!runMethod) {
        errorBox.textContent = "runMethod not configured";
        return;
      }
      let args: unknown[];
      try {
        args = JSON.parse(argsInput.value) as unknown[];
        if (!Array.isArray(args)) throw new Error("args must be a JSON array");
      } catch (e) {
        errorBox.textContent = e instanceof Error ? e.message : String(e);
        return;
      }
      result = await runMethod(source, methodInput.value, args);
    } else {
      if (!runMain) {
        errorBox.textContent = "runMain not configured";
        return;
      }
      result = await runMain(source);
    }
    if (!result.ok && result.error) {
      errorBox.textContent = result.error.message;
      if (result.error.span) {
        applyHighlights(view, [
          { start: result.error.span.start, end: result.error.span.end, kind: "error" },
        ]);
      }
    }
    onRun?.(result);
  });

  return {
    update(next) {
      if (next.timeline && next.timeline !== timeline) {
        unsub();
        timeline = next.timeline;
        unsub = timeline.subscribe((ev) => {
          if (ev.type === "highlight" || ev.type === "tick" || ev.type === "seek") {
            applyHighlights(
              view,
              timeline.highlight(),
              timeline.source || view.state.doc.toString(),
            );
          }
          if (ev.type === "tick") {
            const plugin = view.plugin(chipPlugin);
            plugin?.rebuild(view);
            view.dispatch({});
          }
        });
      }
      if (next.source !== undefined && next.source !== view.state.doc.toString()) {
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: next.source },
        });
      }
      if (next.profile) {
        profile = next.profile;
        profileSel.value = profile;
        syncProfileUi();
      }
      if (next.method !== undefined) {
        method = next.method;
        methodInput.value = method;
      }
      if (next.argsJson !== undefined) {
        argsJson = next.argsJson;
        argsInput.value = argsJson;
      }
      if (next.expectedJson !== undefined) expectedJson = next.expectedJson;
      if (next.onRun) onRun = next.onRun;
      if (next.runMain) runMain = next.runMain;
      if (next.runMethod) runMethod = next.runMethod;
      void expectedJson;
    },
    destroy() {
      unsub();
      view.destroy();
      el.innerHTML = "";
    },
  };
}
