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

  // Single scrubber bar: segment colors + progress + thumb + hover tooltip
  const barWrap = document.createElement("div");
  barWrap.style.cssText = "position:relative;margin:10px 0 4px;padding-top:22px;";
  barWrap.dataset.testid = "seeker-scrubber";

  const bar = document.createElement("div");
  bar.style.cssText =
    "position:relative;height:14px;background:#e2e8f0;border-radius:7px;cursor:pointer;overflow:hidden;user-select:none;";
  bar.dataset.testid = "seeker-track";

  const progress = document.createElement("div");
  progress.style.cssText =
    "position:absolute;left:0;top:0;bottom:0;width:0%;background:#94a3b844;pointer-events:none;z-index:1;";
  progress.dataset.testid = "seeker-progress";

  const thumb = document.createElement("div");
  thumb.style.cssText =
    "position:absolute;top:50%;width:14px;height:14px;margin-left:-7px;margin-top:-7px;left:0%;border-radius:50%;background:#0f172a;border:2px solid #fff;box-shadow:0 1px 3px #0003;pointer-events:none;z-index:3;";
  thumb.dataset.testid = "seeker-thumb";

  const tooltip = document.createElement("div");
  tooltip.style.cssText = [
    "position:absolute",
    "bottom:calc(100% - 18px)",
    "left:0",
    "transform:translateX(-50%)",
    "display:none",
    "z-index:5",
    "max-width:min(420px,80vw)",
    "padding:4px 8px",
    "border-radius:6px",
    "background:#0f172a",
    "color:#f8fafc",
    "font:11px/1.35 ui-monospace,monospace",
    "white-space:nowrap",
    "overflow:hidden",
    "text-overflow:ellipsis",
    "pointer-events:none",
    "box-shadow:0 4px 12px #0004",
  ].join(";");
  tooltip.dataset.testid = "seeker-tooltip";

  bar.append(progress, thumb);
  barWrap.append(tooltip, bar);

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
  for (const s of [1, 10, 30, 60, 120, 240, 500, 1000]) {
    const o = document.createElement("option");
    o.value = String(s);
    o.textContent = `${s}/s`;
    if (s === 120) o.selected = true;
    speed.appendChild(o);
  }
  const indexLabel = document.createElement("span");
  indexLabel.dataset.testid = "seeker-index";
  indexLabel.style.font = "12px monospace";

  controls.append(playBtn, pauseBtn, speed, indexLabel);
  el.append(barWrap, controls);

  let dragging = false;

  function indexFromClientX(clientX: number): number {
    const rect = bar.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (clientX - rect.left) / Math.max(rect.width, 1)));
    return Math.round(ratio * timeline.length);
  }

  function paintSegments(): void {
    bar.querySelectorAll("[data-seg]").forEach((n) => n.remove());
    const len = Math.max(timeline.length, 1);
    for (const seg of buildLoopSegments(timeline.events)) {
      const end = seg.endIndex ?? timeline.length;
      const left = (seg.startIndex / len) * 100;
      const width = Math.max(((end - seg.startIndex + 1) / len) * 100, 0.5);
      const segEl = document.createElement("div");
      segEl.dataset.seg = String(seg.loop_id);
      segEl.style.cssText = `position:absolute;left:${left}%;width:${width}%;top:0;bottom:0;background:${COLORS[seg.loop_id % COLORS.length]};opacity:0.55;z-index:0;pointer-events:none;`;
      bar.insertBefore(segEl, progress);
    }
  }

  function syncUi(): void {
    const len = Math.max(timeline.length, 1);
    const pct = (timeline.index / len) * 100;
    progress.style.width = `${pct}%`;
    thumb.style.left = `${pct}%`;
    indexLabel.textContent = `${timeline.index} / ${timeline.length}`;
    paintSegments();
  }

  function showTooltipAt(clientX: number, t: number): void {
    const rect = bar.getBoundingClientRect();
    const wrapRect = barWrap.getBoundingClientRect();
    const x = clientX - wrapRect.left;
    tooltip.style.left = `${x}px`;
    tooltip.style.display = "block";

    const events = timeline.events;
    const ev = t > 0 ? events[t - 1] : undefined;
    if (ev?.span) {
      const { line, snippet } = lineAtByte(source, ev.span.start);
      tooltip.textContent = `t=${t} · L${line}: ${snippet.slice(0, 72)}`;
      timeline.setHoverHighlight([
        { start: ev.span.start, end: ev.span.end, kind: ev.kind },
      ]);
    } else {
      tooltip.textContent = `t=${t}`;
      timeline.setHoverHighlight(null);
    }
    void rect;
  }

  function hideTooltip(): void {
    tooltip.style.display = "none";
    timeline.setHoverHighlight(null);
  }

  function onPointer(clientX: number, seek: boolean): void {
    const t = indexFromClientX(clientX);
    showTooltipAt(clientX, t);
    if (seek) timeline.seek(t);
  }

  bar.addEventListener("pointerdown", (e) => {
    dragging = true;
    bar.setPointerCapture(e.pointerId);
    onPointer(e.clientX, true);
  });
  bar.addEventListener("pointermove", (e) => {
    if (dragging) onPointer(e.clientX, true);
    else onPointer(e.clientX, false);
  });
  bar.addEventListener("pointerup", (e) => {
    dragging = false;
    try {
      bar.releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
  });
  bar.addEventListener("pointerleave", () => {
    if (!dragging) hideTooltip();
  });

  let unsub = timeline.subscribe((ev) => {
    if (ev.type === "tick" || ev.type === "seek") syncUi();
  });

  playBtn.addEventListener("click", () => {
    timeline.play({ speed: Number(speed.value) });
  });
  pauseBtn.addEventListener("click", () => timeline.pause());

  function isTypingTarget(t: EventTarget | null): boolean {
    if (!(t instanceof HTMLElement)) return false;
    const tag = t.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
    if (t.isContentEditable) return true;
    if (t.closest(".cm-editor")) return true;
    return false;
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (e.code !== "Space" && e.key !== " ") return;
    if (e.repeat) return;
    if (isTypingTarget(e.target)) return;
    e.preventDefault();
    if (timeline.playing) timeline.pause();
    else timeline.play({ speed: Number(speed.value) });
  }
  window.addEventListener("keydown", onKeyDown);

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
      window.removeEventListener("keydown", onKeyDown);
      unsub();
      el.innerHTML = "";
    },
  };
}
