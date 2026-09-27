import "./layout.css";
import { initTheme } from "./theme";
import { loadEditorDraft, saveEditorDraft } from "./editorDraft";
import { createTimeline } from "@rscpp/timeline";
import { mountSeeker } from "@rscpp/seeker";
import { mountEditor } from "@rscpp/editor";
import { mountDsViewer, listAllocIds } from "@rscpp/ds-viewer";
import type { Representation } from "@rscpp/ds-viewer";
import { mountDebugger } from "@rscpp/debugger";
import { initRunner, run, runMethod } from "@rscpp/runner";
import type { RunResult } from "@rscpp/runner";
import { installDebug } from "./debug";

const theme = initTheme();
import twoSum from "../../../packages/timeline/src/fixtures/two_sum.json";
import parentTree from "../../../packages/timeline/src/fixtures/parent_tree.json";
import vectorPush from "../../../packages/timeline/src/fixtures/vector_push.json";
import dfs from "../../../packages/timeline/src/fixtures/dfs.json";
import dfsMain from "../../../packages/timeline/src/fixtures/dfs_main.json";
import validParen from "../../../packages/timeline/src/fixtures/valid_parentheses.json";
import validParenMain from "../../../packages/timeline/src/fixtures/valid_parentheses_main.json";
import nQueens from "../../../packages/timeline/src/fixtures/n_queens.json";
import type { EventJson } from "@rscpp/timeline";

type FixtureMeta = {
  source?: string;
  events: EventJson[];
  ok?: boolean;
  value?: unknown;
  method?: string | null;
  args?: unknown;
  /** Preferred heap object for DS pane */
  preferObjId?: number;
  preferRepr?: Representation;
  profile?: "leetcode" | "codeforces";
};

const root = document.querySelector("#app") ?? document.body;
root.id = "root";
root.innerHTML = `
  <div id="editor-pane" data-testid="editor-pane"></div>
  <div id="ds-pane" data-testid="ds-pane">
    <div style="display:flex;gap:8px;margin-bottom:8px;align-items:center;flex-wrap:wrap;">
      <label>Fixture
        <select data-testid="fixture-select">
          <option value="two_sum">two_sum (56)</option>
          <option value="dfs">dfs countComponents (410)</option>
          <option value="dfs_main">dfs main (444)</option>
          <option value="valid_parentheses">valid_parentheses (270)</option>
          <option value="valid_parentheses_main">valid_paren main (213)</option>
          <option value="n_queens">n_queens (2292)</option>
          <option value="vector_push">vector_push</option>
          <option value="parent_tree">parent_tree</option>
        </select>
      </label>
      <label>Object (single mode)
        <select data-testid="ds-object-picker"></select>
      </label>
    </div>
    <div id="ds-host"></div>
  </div>
  <div id="seeker-pane"></div>
`;

const fixtures: Record<string, FixtureMeta> = {
  two_sum: {
    ...(twoSum as FixtureMeta),
    method: "Solution::twoSum",
    args: [[2, 7, 11, 15], 9],
    preferObjId: 0,
    preferRepr: "array",
  },
  dfs: {
    ...(dfs as FixtureMeta),
    preferObjId: 6,
    preferRepr: "adjacency-list",
  },
  dfs_main: {
    ...(dfsMain as FixtureMeta),
    profile: "codeforces",
    preferObjId: 6,
    preferRepr: "adjacency-list",
  },
  valid_parentheses: {
    ...(validParen as FixtureMeta),
    preferObjId: 2,
    preferRepr: "stack",
  },
  valid_parentheses_main: {
    ...(validParenMain as FixtureMeta),
    profile: "codeforces",
    preferObjId: 2,
    preferRepr: "stack",
  },
  n_queens: {
    ...(nQueens as FixtureMeta),
    method: "Solution::solveNQueens",
    args: [4],
    preferObjId: 0,
    preferRepr: "array",
  },
  vector_push: {
    source: "// vector push fixture",
    events: vectorPush.events as EventJson[],
    preferObjId: 0,
    preferRepr: "array",
  },
  parent_tree: {
    source: "// parent array tree\nvector<int> parent = {-1,0,0,1,1};",
    events: parentTree.events as EventJson[],
    preferObjId: 0,
    preferRepr: "tree",
  },
};

let source = String(twoSum.source ?? "");
let events = twoSum.events as EventJson[];
let draft = loadEditorDraft();
let editorProfile: "leetcode" | "codeforces" = draft?.profile ?? "leetcode";
let editorMethod = draft?.method ?? "Solution::twoSum";
let editorArgsJson = draft?.argsJson ?? "[[2,7,11,15],9]";
if (draft?.source) source = draft.source;

let timeline = createTimeline({ events, source });
let lastRun: RunResult | null = {
  ok: Boolean(twoSum.ok),
  events,
  value: twoSum.value,
};

const seekerEl = document.querySelector("#seeker-pane") as HTMLElement;
const seeker = mountSeeker(seekerEl, { timeline, source });

const debuggerEl = document.createElement("div");
document.body.appendChild(debuggerEl);
const eventDebugger = mountDebugger(debuggerEl, { timeline });

const editorEl = document.querySelector("#editor-pane") as HTMLElement;
let editor: ReturnType<typeof mountEditor>;

const dsHost = document.querySelector("#ds-host") as HTMLElement;
const picker = document.querySelector("[data-testid=ds-object-picker]") as HTMLSelectElement;
const fixtureSel = document.querySelector("[data-testid=fixture-select]") as HTMLSelectElement;

const themeBtn = document.createElement("button");
themeBtn.type = "button";
themeBtn.dataset.testid = "theme-toggle";
function syncThemeLabel(): void {
  const p = theme.getPreference();
  const label =
    p === "system" ? "System" : p === "light" ? "Light" : "Dark";
  themeBtn.textContent = `Theme: ${label}`;
}
syncThemeLabel();
themeBtn.addEventListener("click", () => {
  theme.cycle();
  syncThemeLabel();
});
const dsToolbar = dsHost.previousElementSibling;
if (dsToolbar) dsToolbar.appendChild(themeBtn);

let ds = mountDsViewer(dsHost, { timeline, objId: null, mode: "all" });

function refreshPicker(preferObjId?: number, preferRepr?: Representation): void {
  const allocs = listAllocIds(timeline);
  picker.innerHTML = "";
  const none = document.createElement("option");
  none.value = "";
  none.textContent = "(none)";
  picker.appendChild(none);
  for (const a of allocs) {
    const o = document.createElement("option");
    o.value = String(a.id);
    o.textContent = `#${a.id} ${a.type_name}`;
    picker.appendChild(o);
  }
  const pick =
    preferObjId !== undefined && allocs.some((a) => a.id === preferObjId)
      ? preferObjId
      : allocs[0]?.id;
  if (pick !== undefined) {
    picker.value = String(pick);
    ds.update({
      objId: pick,
      mode: "all",
      ...(preferRepr ? { representation: preferRepr } : {}),
    });
  } else {
    ds.update({ mode: "all" });
  }
}

function wireDebug(): void {
  installDebug({
    timeline,
    get lastRun() {
      return lastRun;
    },
    seek: (t) => timeline.seek(t),
    snapshot: () => timeline.snapshot(),
    highlight: () => timeline.highlight(),
  });
}

function seekShowingObject(objId?: number): void {
  if (objId === undefined) {
    timeline.seek(timeline.length);
    return;
  }
  let alive = false;
  let lastAlive = 0;
  timeline.events.forEach((e, i) => {
    if (e.kind === "Alloc" && e.id === objId) {
      alive = true;
      lastAlive = i + 1;
    } else if (e.kind === "Dealloc" && e.id === objId) {
      alive = false;
    } else if (alive) {
      lastAlive = i + 1;
    }
  });
  timeline.seek(lastAlive > 0 ? lastAlive : timeline.length);
}

function remountTimeline(
  next: {
    source: string;
    events: EventJson[];
    run: RunResult | null;
    preferObjId?: number;
    preferRepr?: Representation;
  },
): void {
  timeline.pause();
  source = next.source;
  events = next.events;
  lastRun = next.run;
  timeline = createTimeline({ events, source });
  seeker.update({ timeline, source });
  editor.update({ timeline, source });
  eventDebugger.update({ timeline });
  ds.update({ timeline, objId: null });
  seekShowingObject(next.preferObjId);
  refreshPicker(next.preferObjId, next.preferRepr);
  wireDebug();
}

editor = mountEditor(editorEl, {
  timeline,
  source,
  profile: editorProfile,
  method: editorMethod,
  argsJson: editorArgsJson,
  runMain: (src) => run(src),
  runMethod: (src, method, args) => runMethod(src, method, args),
  onDraftChange: (d) => {
    editorProfile = d.profile;
    editorMethod = d.method;
    editorArgsJson = d.argsJson;
    saveEditorDraft(d);
  },
  onRun: (result) => {
    remountTimeline({
      source: editor.getSource(),
      events: result.events,
      run: result,
    });
  },
});

// Persist initial draft so a blank storage gets the starting values.
saveEditorDraft({
  source: editor.getSource(),
  method: editorMethod,
  argsJson: editorArgsJson,
  profile: editorProfile,
});

picker.addEventListener("change", () => {
  const id = picker.value === "" ? null : Number(picker.value);
  ds.update({ objId: id });
});

fixtureSel.addEventListener("change", () => {
  const fx = fixtures[fixtureSel.value];
  if (!fx) return;
  remountTimeline({
    source: String(fx.source ?? ""),
    events: fx.events,
    run: { ok: Boolean(fx.ok ?? true), events: fx.events, value: fx.value },
    preferObjId: fx.preferObjId,
    preferRepr: fx.preferRepr,
  });
  editor.update({
    source: String(fx.source ?? ""),
    profile: fx.profile ?? "leetcode",
    method: fx.method ?? "Solution::twoSum",
    argsJson: fx.args !== undefined && fx.args !== null ? JSON.stringify(fx.args) : "[]",
  });
  saveEditorDraft({
    source: String(fx.source ?? ""),
    profile: fx.profile ?? "leetcode",
    method: fx.method ?? "Solution::twoSum",
    argsJson: fx.args !== undefined && fx.args !== null ? JSON.stringify(fx.args) : "[]",
  });
});

refreshPicker(0, "array");
wireDebug();

void initRunner().then(() => {
  const status = document.createElement("div");
  status.dataset.testid = "wasm-status";
  status.textContent = "wasm ready";
  status.style.cssText =
    "position:fixed;right:64px;top:12px;font:11px monospace;opacity:0.6;z-index:9998;";
  document.body.appendChild(status);
});

timeline.seek(timeline.length);
refreshPicker(0, "array");
