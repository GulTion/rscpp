import "./layout.css";
import { createTimeline } from "@rscpp/timeline";
import { mountSeeker } from "@rscpp/seeker";
import { mountEditor } from "@rscpp/editor";
import { initRunner, run, runMethod } from "@rscpp/runner";
import type { RunResult } from "@rscpp/runner";
import { installDebug } from "./debug";
import twoSum from "../../../packages/timeline/src/fixtures/two_sum.json";
import type { EventJson } from "@rscpp/timeline";

const root = document.querySelector("#app") ?? document.body;
root.id = "root";
root.innerHTML = `
  <div id="editor-pane" data-testid="editor-pane"></div>
  <div id="ds-pane" data-testid="ds-pane"><p data-testid="ds-placeholder">DS viewer coming soon</p></div>
  <div id="seeker-pane"></div>
`;

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

function remountTimeline(next: { source: string; events: EventJson[]; run: RunResult }) {
  timeline.pause();
  source = next.source;
  events = next.events;
  lastRun = next.run;
  timeline = createTimeline({ events, source });
  seeker.update({ timeline, source });
  editor.update({ timeline, source });
  timeline.seek(0);
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

installDebug({
  timeline,
  get lastRun() {
    return lastRun;
  },
  seek: (t) => timeline.seek(t),
  snapshot: () => timeline.snapshot(),
  highlight: () => timeline.highlight(),
});

void initRunner().then(() => {
  const status = document.createElement("div");
  status.dataset.testid = "wasm-status";
  status.textContent = "wasm ready";
  status.style.cssText = "position:fixed;right:8px;top:8px;font:11px monospace;opacity:0.6;";
  document.body.appendChild(status);
});

timeline.seek(0);
