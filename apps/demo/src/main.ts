import "./layout.css";
import { createTimeline } from "@rscpp/timeline";
import { mountSeeker } from "@rscpp/seeker";
import { installDebug } from "./debug";
import twoSum from "../../../packages/timeline/src/fixtures/two_sum.json";
import type { EventJson } from "@rscpp/timeline";
import type { RunResult } from "@rscpp/runner";

const root = document.querySelector("#app") ?? document.body;
root.id = "root";
root.innerHTML = `
  <div id="editor-pane" data-testid="editor-pane"><pre id="source-view"></pre></div>
  <div id="ds-pane" data-testid="ds-pane"><p>DS viewer coming soon</p></div>
  <div id="seeker-pane"></div>
`;

const source = String(twoSum.source ?? "");
const events = twoSum.events as EventJson[];
const timeline = createTimeline({ events, source });

const sourceView = document.querySelector("#source-view")!;
sourceView.textContent = source;

const seekerEl = document.querySelector("#seeker-pane") as HTMLElement;
mountSeeker(seekerEl, { timeline, source });

let lastRun: RunResult | null = {
  ok: Boolean(twoSum.ok),
  events,
  value: twoSum.value,
};

installDebug({
  timeline,
  get lastRun() {
    return lastRun;
  },
  seek: (t) => timeline.seek(t),
  snapshot: () => timeline.snapshot(),
  highlight: () => timeline.highlight(),
});

timeline.seek(0);
