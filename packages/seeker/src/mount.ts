import type { Timeline } from "@rscpp/timeline";
import { buildLoopSegments } from "./segments.js";

export type MountHandle = {
  update(props: Partial<SeekerProps>): void;
  destroy(): void;
};

export type SeekerProps = {
  timeline: Timeline;
  source: string;
};

const COLORS = [
  "#4e79a7",
  "#f28e2b",
  "#e15759",
  "#76b7b2",
  "#59a14f",
  "#edc948",
  "#b07aa1",
];

function lineAtByte(source: string, offset: number): { line: number; snippet: string } {
  let line = 1;
  let start = 0;
  for (let i = 0; i < offset && i < source.length; i++) {
    if (source[i] === "\n") {
      line++;
      start = i + 1;
    }
  }
  const end = source.indexOf("\n", start);
  const snippet = source.slice(start, end === -1 ? undefined : end).trim();
  return { line, snippet };
}

export function mountSeeker(el: HTMLElement, props: SeekerProps): MountHandle {
  let { timeline, source } = props;
  el.dataset.testid = "seeker-root";
  el.innerHTML = "";

  const track = document.createElement("div");
  track.style.cssText =
    "position:relative;height:12px;background:#ddd;border-radius:4px;margin:8px 0;";
  track.dataset.testid = "seeker-track";

  const scrubber = document.createElement("input");
  scrubber.type = "range";
  scrubber.min = "0";
  scrubber.dataset.testid = "seeker-scrubber";
  scrubber.style.cssText = "width:100%;position:relative;z-index:2;";

  const tooltip = document.createElement("div");
  tooltip.style.cssText =
    "font:12px monospace;min-height:1.2em;color:#333;margin-top:4px;";
  tooltip.dataset.testid = "seeker-tooltip";

  const controls = document.createElement("div");
  controls.style.cssText = "display:flex;gap:8px;align-items:center;flex-wrap:wrap;";

  const playBtn = document.createElement("button");
  playBtn.textContent = "Play";
  playBtn.dataset.testid = "seeker-play";
  const pauseBtn = document.createElement("button");
  pauseBtn.textContent = "Pause";
  pauseBtn.dataset.testid = "seeker-pause";
  const speed = document.createElement("select");
  speed.dataset.testid = "seeker-speed";
  for (const s of [10, 30, 60, 120]) {
    const o = document.createElement("option");
    o.value = String(s);
    o.textContent = `${s}/s`;
    if (s === 30) o.selected = true;
    speed.appendChild(o);
  }
  const indexLabel = document.createElement("span");
  indexLabel.dataset.testid = "seeker-index";
  indexLabel.style.font = "12px monospace";

  controls.append(playBtn, pauseBtn, speed, indexLabel);
  el.append(track, scrubber, tooltip, controls);

  function paintSegments(): void {
    track.querySelectorAll("[data-seg]").forEach((n) => n.remove());
    const len = Math.max(timeline.length, 1);
    for (const seg of buildLoopSegments(timeline.events)) {
      const end = seg.endIndex ?? timeline.length;
      const left = (seg.startIndex / len) * 100;
      const width = Math.max(((end - seg.startIndex + 1) / len) * 100, 0.5);
      const bar = document.createElement("div");
      bar.dataset.seg = String(seg.loop_id);
      bar.style.cssText = `position:absolute;left:${left}%;width:${width}%;top:0;bottom:0;background:${COLORS[seg.loop_id % COLORS.length]};opacity:0.45;border-radius:2px;z-index:1;pointer-events:none;`;
      track.appendChild(bar);
    }
  }

  function syncUi(): void {
    scrubber.max = String(timeline.length);
    scrubber.value = String(timeline.index);
    indexLabel.textContent = `${timeline.index} / ${timeline.length}`;
    paintSegments();
  }

  let unsub = timeline.subscribe((ev) => {
    if (ev.type === "tick" || ev.type === "seek") syncUi();
  });

  scrubber.addEventListener("input", () => {
    timeline.seek(Number(scrubber.value));
  });

  scrubber.addEventListener("mousemove", (e) => {
    const rect = scrubber.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    const t = Math.round(ratio * timeline.length);
    const events = timeline.events;
    const ev = t > 0 ? events[t - 1] : undefined;
    if (ev?.span) {
      const { line, snippet } = lineAtByte(source, ev.span.start);
      tooltip.textContent = `L${line}: ${snippet.slice(0, 80)}`;
      timeline.setHoverHighlight([
        { start: ev.span.start, end: ev.span.end, kind: ev.kind },
      ]);
    } else {
      tooltip.textContent = `t=${t}`;
      timeline.setHoverHighlight(null);
    }
  });

  scrubber.addEventListener("mouseleave", () => {
    timeline.setHoverHighlight(null);
    tooltip.textContent = "";
  });

  playBtn.addEventListener("click", () => {
    timeline.play({ speed: Number(speed.value) });
  });
  pauseBtn.addEventListener("click", () => timeline.pause());

  syncUi();

  return {
    update(next) {
      if (next.timeline && next.timeline !== timeline) {
        unsub();
        timeline = next.timeline;
        unsub = timeline.subscribe((ev) => {
          if (ev.type === "tick" || ev.type === "seek") syncUi();
        });
      }
      if (next.source !== undefined) source = next.source;
      syncUi();
    },
    destroy() {
      timeline.pause();
      unsub();
      el.innerHTML = "";
    },
  };
}
