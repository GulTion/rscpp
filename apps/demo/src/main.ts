import "./layout.css";
import { createTimeline } from "@rscpp/timeline";
import { mountSeeker } from "@rscpp/seeker";
import { mountEditor } from "@rscpp/editor";
import { mountDsViewer, listAllocIds } from "@rscpp/ds-viewer";
import { initRunner, run, runMethod } from "@rscpp/runner";
import type { RunResult } from "@rscpp/runner";
import { installDebug } from "./debug";
import twoSum from "../../../packages/timeline/src/fixtures/two_sum.json";
import parentTree from "../../../packages/timeline/src/fixtures/parent_tree.json";
import vectorPush from "../../../packages/timeline/src/fixtures/vector_push.json";
import type { EventJson } from "@rscpp/timeline";

const root = document.querySelector("#app") ?? document.body;
root.id = "root";
root.innerHTML = `
  <div id="editor-pane" data-testid="editor-pane"></div>
  <div id="ds-pane" data-testid="ds-pane">
    <div style="display:flex;gap:8px;margin-bottom:8px;align-items:center;">
      <label>Fixture
        <select data-testid="fixture-select">
          <option value="two_sum">two_sum</option>
          <option value="vector_push">vector_push</option>
          <option value="parent_tree">parent_tree</option>
        </select>
      </label>
      <label>Object
        <select data-testid="ds-object-picker"></select>
      </label>
    </div>
    <div id="ds-host"></div>
  </div>
  <div id="seeker-pane"></div>
`;

const fixtures: Record<string, { source?: string; events: EventJson[]; ok?: boolean; value?: unknown }> = {
  two_sum: twoSum as never,
  vector_push: { source: "// vector push fixture", events: vectorPush.events as EventJson[] },
  parent_tree: {
    source: "// parent array tree\nvector<int> parent = {-1,0,0,1,1};",
    events: parentTree.events as EventJson[],
  },
};

let source = String(twoSum.source ?? "");
let events = twoSum.events as EventJson[];
let timeline = createTimeline({ events, source });
let lastRun: RunResult | null = {
  ok: Boolean(twoSum.ok),
  events,
  value: twoSum.value,
};

const seekerEl = document.querySelector("#seeker-pane") as HTMLElement;
const seeker = mountSeeker(seekerEl, { timeline, source });

const editorEl = document.querySelector("#editor-pane") as HTMLElement;
let editor: ReturnType<typeof mountEditor>;

const dsHost = document.querySelector("#ds-host") as HTMLElement;
const picker = document.querySelector("[data-testid=ds-object-picker]") as HTMLSelectElement;
const fixtureSel = document.querySelector("[data-testid=fixture-select]") as HTMLSelectElement;

let ds = mountDsViewer(dsHost, { timeline, objId: null });

function refreshPicker(): void {
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
  if (allocs[0]) {
    picker.value = String(allocs[0].id);
    ds.update({ objId: allocs[0].id });
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

function remountTimeline(next: { source: string; events: EventJson[]; run: RunResult | null }) {
  timeline.pause();
  source = next.source;
  events = next.events;
  lastRun = next.run;
  timeline = createTimeline({ events, source });
  seeker.update({ timeline, source });
  editor.update({ timeline, source });
  ds.update({ timeline, objId: null });
  refreshPicker();
  timeline.seek(0);
  wireDebug();
}

editor = mountEditor(editorEl, {
  timeline,
  source,
  profile: "leetcode",
  method: "Solution::twoSum",
  argsJson: "[[2,7,11,15],9]",
  runMain: (src) => run(src),
  runMethod: (src, method, args) => runMethod(src, method, args),
  onRun: (result) => {
    const doc = editorEl.querySelector(".cm-content")?.textContent ?? source;
    remountTimeline({
      source: doc,
      events: result.events,
      run: result,
    });
  },
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
  });
  editor.update({ source: String(fx.source ?? "") });
});

refreshPicker();
wireDebug();

void initRunner().then(() => {
  const status = document.createElement("div");
  status.dataset.testid = "wasm-status";
  status.textContent = "wasm ready";
  status.style.cssText =
    "position:fixed;right:8px;top:8px;font:11px monospace;opacity:0.6;";
  document.body.appendChild(status);
});

timeline.seek(Math.min(timeline.length, 1));
