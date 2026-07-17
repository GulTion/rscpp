import { TIMELINE_VERSION, createTimeline } from "@rscpp/timeline";
import twoSum from "../../../packages/timeline/src/fixtures/two_sum.json";

const app = document.querySelector("#app") ?? document.body;
const tl = createTimeline({
  events: twoSum.events as import("@rscpp/timeline").EventJson[],
  source: String(twoSum.source ?? ""),
});
tl.seek(tl.length);

app.innerHTML = `
  <p data-testid="demo-status">timeline v${TIMELINE_VERSION} · events ${tl.length} · objects ${tl.snapshot().objects.size}</p>
`;
