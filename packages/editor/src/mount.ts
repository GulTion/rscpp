import {
  EditorView,
  Decoration,
  ViewPlugin,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { Compartment, EditorState } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { cpp } from "@codemirror/lang-cpp";
import type { Timeline } from "@rscpp/timeline";
import type { RunResult } from "@rscpp/runner";
import { applyHighlights, highlightField } from "./decorations.js";
import { buildChipDecos } from "./chipDecos.js";
import { buildByteIndexMap, jsToByte } from "./spans.js";
import { eventIndexForLine } from "./lineSeek.js";
import {
  editorTheme,
  readDocumentTheme,
  watchDocumentTheme,
} from "./theme.js";

export type MountHandle = {
  update(props: Partial<EditorProps>): void;
  /** Current editor document (preserves newlines). */
  getSource(): string;
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
  toolbar.style.cssText =
    "display:flex;flex-wrap:wrap;gap:6px;margin-bottom:6px;align-items:center;";

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
  cmHost.style.cssText =
    "height:calc(100% - 80px);min-height:200px;border:1px solid #e5e7eb;";

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
        this.decorations = buildChipDecos(view.state, timeline);
      }
      update(u: { view: EditorView; docChanged: boolean }) {
        if (u.docChanged) this.rebuild(u.view);
      }
    },
    { decorations: (v) => v.decorations },
  );

  const themeComp = new Compartment();
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
        themeComp.of(editorTheme(readDocumentTheme())),
        EditorView.domEventHandlers({
          mousedown: (e, v) => {
            const t = e.target as HTMLElement | null;
            if (!t?.closest?.(".cm-gutters")) return false;
            e.preventDefault();
            const pos = v.posAtCoords({ x: e.clientX, y: e.clientY });
            if (pos == null) return true;
            const line = v.state.doc.lineAt(pos);
            const lineIndex0 = line.number - 1;
            const doc = v.state.doc.toString();
            const source = timeline.source || doc;
            const idx = eventIndexForLine(timeline.events, source, lineIndex0);
            if (idx !== null) timeline.seek(idx + 1);
            return true;
          },
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
    applyHighlights(
      view,
      timeline.highlight(),
      timeline.source || view.state.doc.toString(),
    );
  }

  function paintChips(): void {
    const plugin = view.plugin(chipPlugin);
    plugin?.rebuild(view);
    view.dispatch({});
  }

  function onTimeline(): void {
    paintHighlights();
    paintChips();
  }

  let unsub = timeline.subscribe((ev) => {
    if (ev.type === "highlight" || ev.type === "tick" || ev.type === "seek") {
      onTimeline();
    }
  });
  const unwatchTheme = watchDocumentTheme((mode) => {
    view.dispatch({ effects: themeComp.reconfigure(editorTheme(mode)) });
  });
  onTimeline();

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
        applyHighlights(
          view,
          [{ start: result.error.span.start, end: result.error.span.end, kind: "error" }],
          source,
        );
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
            onTimeline();
          }
        });
        onTimeline();
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
    getSource() {
      return view.state.doc.toString();
    },
    destroy() {
      unwatchTheme();
      unsub();
      view.destroy();
      el.innerHTML = "";
    },
  };
}
