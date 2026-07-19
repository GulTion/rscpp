import type { Timeline } from "@rscpp/timeline";
import {
  buildCallSegments,
  buildLoopSegments,
  colorIndexForName,
  currentSegment,
} from "./segments.js";
import {
  countVisibleEvents,
  eventIndexToVisual,
  playheadToVisual,
  visualToPlayhead,
} from "./visibleAxis.js";
import {
  countVisibleInRange,
  playheadToVisualInRange,
  visualToPlayheadInRange,
} from "./detailAxis.js";
import {
  LAYER_H,
  allSpans,
  callStackDepth,
  loopStackDepth,
  maxStackDepth,
  slabBottom,
  stackHeight,
} from "./stackLayout.js";

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

function shortFnName(name: string): string {
  const i = name.lastIndexOf("::");
  return i >= 0 ? name.slice(i + 2) : name;
}

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

function ensureSeekerStyle(): void {
  if (typeof document === "undefined") return;
  let style = document.getElementById("seeker-style") as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement("style");
    style.id = "seeker-style";
    document.head?.appendChild(style);
  }
  // Always refresh so HMR / remount picks up style fixes.
  style.textContent = `
.seeker-slab-active {
  outline: 2px solid var(--thumb, #0f172a);
  outline-offset: -1px;
  filter: brightness(1.08);
}
.seeker-thumb::after {
  content: "";
  position: absolute;
  top: -5px;
  left: 50%;
  transform: translateX(-50%);
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--thumb, #0f172a);
  border: 2px solid var(--panel, #fff);
  box-shadow: 0 1px 3px var(--shadow, #0003);
}
:root[data-theme="dark"] .seeker-slab-active {
  filter: brightness(1.2);
}
`;
}

export function mountSeeker(el: HTMLElement, props: SeekerProps): MountHandle {
  let { timeline, source } = props;
  ensureSeekerStyle();
  el.dataset.testid = "seeker-root";
  el.innerHTML = "";

  // Stacked scrubber: depth layers + progress + thumb + per-segment start labels
  const barWrap = document.createElement("div");
  barWrap.style.cssText = "position:relative;margin:8px 0 4px;";
  barWrap.dataset.testid = "seeker-scrubber";

  const markers = document.createElement("div");
  markers.style.cssText = [
    "position:absolute",
    "left:4px",
    "right:4px",
    "top:3px",
    "bottom:5px",
    "pointer-events:none",
    "z-index:5",
    "overflow:visible",
  ].join(";");
  markers.dataset.testid = "seeker-start-labels";

  const bar = document.createElement("div");
  bar.dataset.testid = "seeker-track";
  bar.style.cssText = [
    "position:relative",
    "background:var(--muted, #94a3b8)",
    "border-radius:8px",
    "cursor:pointer",
    "overflow:visible",
    "user-select:none",
    "padding:3px 4px 5px",
    "box-sizing:border-box",
    "box-shadow:inset 0 1px 2px var(--shadow, #0002)",
  ].join(";");

  const stackInner = document.createElement("div");
  stackInner.dataset.testid = "seeker-stack";
  stackInner.style.cssText = [
    "position:relative",
    "width:100%",
    "z-index:1",
  ].join(";");

  const progress = document.createElement("div");
  progress.dataset.testid = "seeker-progress";
  progress.style.cssText = [
    "position:absolute",
    "left:0",
    "top:0",
    "bottom:0",
    "width:0%",
    "background:color-mix(in srgb, var(--thumb, #0f172a) 12%, transparent)",
    "pointer-events:none",
    "z-index:2",
    "border-radius:8px 0 0 8px",
  ].join(";");

  const thumb = document.createElement("div");
  thumb.className = "seeker-thumb";
  thumb.dataset.testid = "seeker-thumb";
  thumb.style.cssText = [
    "position:absolute",
    "top:0",
    "bottom:0",
    "width:3px",
    "margin-left:-1px",
    "left:0%",
    "background:var(--thumb, #0f172a)",
    "pointer-events:none",
    "z-index:3",
    "box-shadow:0 0 0 2px var(--panel, #fff),0 1px 4px var(--shadow, #0003)",
  ].join(";");

  const tooltip = document.createElement("div");
  tooltip.style.cssText = [
    "position:absolute",
    "bottom:calc(100% - 4px)",
    "left:0",
    "transform:translateX(-50%)",
    "display:none",
    "z-index:6",
    "max-width:min(420px,80vw)",
    "padding:4px 8px",
    "border-radius:6px",
    "background:var(--panel, #0f172a)",
    "color:var(--text, #f8fafc)",
    "border:1px solid var(--border, transparent)",
    "font:11px/1.35 ui-monospace,monospace",
    "white-space:nowrap",
    "overflow:hidden",
    "text-overflow:ellipsis",
    "pointer-events:none",
    "box-shadow:0 4px 12px var(--shadow, #0004)",
  ].join(";");
  tooltip.dataset.testid = "seeker-tooltip";

  bar.append(stackInner, progress, thumb, markers);
  barWrap.append(tooltip, bar);

  // Detail seekbar: zooms to innermost call/loop segment under the playhead
  const detailWrap = document.createElement("div");
  detailWrap.style.cssText = "margin:2px 0 4px;display:none;";
  detailWrap.dataset.testid = "seeker-detail";
  const detailLabel = document.createElement("div");
  detailLabel.dataset.testid = "seeker-detail-label";
  detailLabel.style.cssText =
    "font:11px ui-sans-serif,system-ui,sans-serif;color:var(--muted, #475569);margin-bottom:4px;";
  const detailBar = document.createElement("div");
  detailBar.style.cssText =
    "position:relative;height:14px;background:var(--track, #e2e8f0);border-radius:5px;cursor:pointer;overflow:hidden;user-select:none;";
  detailBar.dataset.testid = "seeker-detail-track";
  const detailSeg = document.createElement("div");
  detailSeg.style.cssText =
    "position:absolute;inset:0;opacity:0.85;pointer-events:none;";
  detailSeg.dataset.testid = "seeker-detail-seg";
  const detailProgress = document.createElement("div");
  detailProgress.style.cssText =
    "position:absolute;left:0;top:0;bottom:0;width:0%;background:color-mix(in srgb, var(--muted, #94a3b8) 27%, transparent);pointer-events:none;z-index:1;";
  const detailThumb = document.createElement("div");
  detailThumb.style.cssText =
    "position:absolute;top:50%;width:10px;height:10px;margin-left:-5px;margin-top:-5px;left:0%;border-radius:50%;background:var(--thumb, #0f172a);border:2px solid var(--panel, #fff);box-shadow:0 1px 2px var(--shadow, #0003);pointer-events:none;z-index:3;";
  detailBar.append(detailSeg, detailProgress, detailThumb);
  detailWrap.append(detailLabel, detailBar);

  const controls = document.createElement("div");
  controls.style.cssText = "display:flex;gap:8px;align-items:center;flex-wrap:wrap;";

  const playBtn = document.createElement("button");
  playBtn.textContent = "Play";
  playBtn.dataset.testid = "seeker-play";
  const pauseBtn = document.createElement("button");
  pauseBtn.textContent = "Pause";
  pauseBtn.dataset.testid = "seeker-pause";

  const speedWrap = document.createElement("label");
  speedWrap.style.cssText =
    "display:inline-flex;align-items:center;gap:6px;font:12px ui-sans-serif,system-ui,sans-serif;color:var(--text, #334155);";
  speedWrap.appendChild(document.createTextNode("Speed"));
  const speed = document.createElement("input");
  speed.type = "range";
  speed.min = "1";
  speed.max = "100";
  speed.step = "1";
  speed.value = "30";
  speed.dataset.testid = "seeker-speed";
  speed.style.cssText = "width:120px;accent-color:var(--thumb, #0f172a);";
  const speedVal = document.createElement("span");
  speedVal.dataset.testid = "seeker-speed-label";
  speedVal.style.cssText = "font:12px monospace;min-width:3.5rem;";
  speedVal.textContent = `${speed.value}/s`;
  speed.addEventListener("input", () => {
    speedVal.textContent = `${speed.value}/s`;
  });
  speedWrap.append(speed, speedVal);

  const indexLabel = document.createElement("span");
  indexLabel.dataset.testid = "seeker-index";
  indexLabel.style.font = "12px monospace";

  const modeBtn = document.createElement("button");
  modeBtn.type = "button";
  modeBtn.dataset.testid = "seeker-mode-toggle";
  modeBtn.style.cssText =
    "font:12px ui-sans-serif,system-ui,sans-serif;padding:2px 8px;border:1px solid var(--control-border, #cbd5e1);border-radius:4px;background:var(--control-bg, #fff);color:var(--text, #334155);cursor:pointer;";

  controls.append(playBtn, pauseBtn, speedWrap, indexLabel, modeBtn);
  const legend = document.createElement("div");
  legend.dataset.testid = "seeker-fn-legend";
  legend.style.cssText =
    "display:flex;flex-wrap:wrap;gap:6px 10px;align-items:center;font:11px ui-sans-serif,system-ui,sans-serif;color:var(--text, #334155);margin-top:6px;";
  el.append(barWrap, detailWrap, controls, legend);

  /** Advanced = stacked segments + detail bar; simple = flat scrubber only. */
  let advanced = true;
  let dragging = false;
  let detailDragging = false;
  /** Continuous visual playhead on the main axis (visible-event units). */
  let visPos = 0;
  let smoothPlaying = false;
  let rafId: number | null = null;
  let lastRaf = 0;
  let lastPaintAt = 0;

  function ratioFromClientX(track: HTMLElement, clientX: number): number {
    const rect = track.getBoundingClientRect();
    return Math.min(1, Math.max(0, (clientX - rect.left) / Math.max(rect.width, 1)));
  }

  function setMainScrubPct(pct: number): void {
    const p = Math.min(100, Math.max(0, pct));
    progress.style.transition = "none";
    thumb.style.transition = "none";
    progress.style.width = `${p}%`;
    thumb.style.left = `${p}%`;
  }

  function setDetailScrubPct(pct: number): void {
    const p = Math.min(100, Math.max(0, pct));
    detailProgress.style.transition = "none";
    detailThumb.style.transition = "none";
    detailProgress.style.width = `${p}%`;
    detailThumb.style.left = `${p}%`;
  }

  function syncDetailFromVisPos(): void {
    if (!advanced) {
      detailWrap.style.display = "none";
      return;
    }
    const events = timeline.events;
    const seg = currentSegment(events, timeline.index);
    if (!seg) {
      detailWrap.style.display = "none";
      return;
    }
    detailWrap.style.display = "block";
    const color =
      seg.kind === "loop"
        ? COLORS[(seg.loop_id ?? 0) % COLORS.length]
        : COLORS[colorIndexForName(seg.name ?? seg.label, COLORS.length)];
    detailSeg.style.background = color;
    const v0 = eventIndexToVisual(events, seg.startIndex);
    const v1 = eventIndexToVisual(events, seg.endIndex + 1);
    const span = Math.max(v1 - v0, 1e-6);
    const local = Math.min(span, Math.max(0, visPos - v0));
    setDetailScrubPct((local / span) * 100);
    const visLen = Math.max(
      countVisibleInRange(events, seg.startIndex, seg.endIndex),
      1,
    );
    const visIdx = playheadToVisualInRange(
      events,
      timeline.index,
      seg.startIndex,
      seg.endIndex,
    );
    detailLabel.textContent = `${seg.label} · ${visIdx} / ${visLen}`;
  }

  function stopSmoothPlay(): void {
    smoothPlaying = false;
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    timeline.pause();
  }

  function startSmoothPlay(): void {
    stopSmoothPlay();
    const events = timeline.events;
    visPos = playheadToVisual(events, timeline.index);
    smoothPlaying = true;
    lastRaf = performance.now();
    const tick = (now: number) => {
      if (!smoothPlaying) return;
      const dt = Math.min(0.05, (now - lastRaf) / 1000);
      lastRaf = now;
      const spd = Math.max(1, Number(speed.value));
      const visLen = Math.max(countVisibleEvents(events), 1);
      visPos += spd * dt;
      if (visPos >= visLen) {
        visPos = visLen;
        setMainScrubPct(100);
        timeline.seek(events.length);
        syncDetailFromVisPos();
        stopSmoothPlay();
        return;
      }
      setMainScrubPct((visPos / visLen) * 100);
      const t = visualToPlayhead(events, visPos);
      if (t !== timeline.index) timeline.seek(t);
      syncDetailFromVisPos();
      indexLabel.textContent = `${Math.min(Math.floor(visPos), visLen)} / ${visLen}`;
      rafId = requestAnimationFrame(tick);
    };
    rafId = requestAnimationFrame(tick);
  }

  function indexFromClientX(clientX: number): number {
    const ratio = ratioFromClientX(bar, clientX);
    const visLen = Math.max(countVisibleEvents(timeline.events), 1);
    return visualToPlayhead(timeline.events, ratio * visLen);
  }

  function makeSlab(
    left: number,
    width: number,
    color: number | string,
    stackDepth: number,
    opts: {
      seg: string;
      title: string;
      active: boolean;
      opacity?: number;
      extra?: Record<string, string>;
    },
  ): HTMLElement {
    const segEl = document.createElement("div");
    segEl.className = "seeker-slab" + (opts.active ? " seeker-slab-active" : "");
    segEl.dataset.seg = opts.seg;
    segEl.dataset.depth = String(stackDepth);
    segEl.dataset.stackDepth = String(stackDepth);
    segEl.title = opts.title;
    if (opts.extra) {
      for (const [k, v] of Object.entries(opts.extra)) segEl.dataset[k] = v;
    }
    const bg = typeof color === "string" ? color : COLORS[color % COLORS.length];
    // Inline all layout — do not rely on stylesheet for visibility.
    segEl.style.cssText = [
      "position:absolute",
      `left:${left}%`,
      `width:${Math.max(width, 0.8)}%`,
      `bottom:${slabBottom(stackDepth)}px`,
      `height:${LAYER_H - 1}px`,
      `background:${bg}`,
      `opacity:${opts.opacity ?? 0.95}`,
      `z-index:${10 + stackDepth}`,
      "pointer-events:none",
      "box-sizing:border-box",
      "border-radius:3px",
      // Fake 3D edge without rotateX (which was hiding the slabs)
      "box-shadow:2px 0 0 rgba(0,0,0,0.15),0 2px 0 rgba(0,0,0,0.2)",
    ].join(";");
    return segEl;
  }

  /** Static label drawn on the segment, flush at its left/start edge. */
  function placeStartLabel(opts: {
    leftPct: number;
    stackDepth: number;
    label: string;
    color: string;
    testid: string;
    title: string;
    seekTo: number;
    eventIndex: number;
  }): void {
    const tip = document.createElement("button");
    tip.type = "button";
    tip.dataset.testid = opts.testid;
    tip.title = opts.title;
    tip.setAttribute("aria-label", opts.title);
    // On the segment band, at its start
    const bottom = slabBottom(opts.stackDepth);
    tip.style.cssText = [
      "position:absolute",
      `left:${opts.leftPct}%`,
      `bottom:${bottom}px`,
      `height:${LAYER_H - 1}px`,
      "transform:none",
      "pointer-events:auto",
      "cursor:pointer",
      "border:none",
      "padding:0",
      "margin:0",
      "background:transparent",
      "display:flex",
      "align-items:center",
      `z-index:${40 + opts.stackDepth}`,
    ].join(";");

    const pill = document.createElement("span");
    pill.textContent = opts.label;
    pill.style.cssText = [
      "font:8px/1 ui-sans-serif,system-ui,sans-serif",
      "font-weight:700",
      "padding:0 4px",
      "height:100%",
      "display:inline-flex",
      "align-items:center",
      "background:transparent",
      "color:#fff",
      "white-space:nowrap",
      "max-width:64px",
      "overflow:hidden",
      "text-overflow:ellipsis",
      "text-shadow:0 0 2px #000,0 1px 2px #000a",
    ].join(";");

    tip.appendChild(pill);
    tip.addEventListener("click", (e) => {
      e.stopPropagation();
      timeline.seek(opts.seekTo);
    });
    tip.addEventListener("pointerenter", (e) => {
      e.stopPropagation();
      const rect = tip.getBoundingClientRect();
      const wrapRect = barWrap.getBoundingClientRect();
      tooltip.style.left = `${rect.left + rect.width / 2 - wrapRect.left}px`;
      tooltip.style.display = "block";
      tooltip.textContent = opts.title;
      const ev = timeline.events[opts.eventIndex];
      if (ev?.span) {
        timeline.setHoverHighlight([
          { start: ev.span.start, end: ev.span.end, kind: ev.kind },
        ]);
      }
    });
    tip.addEventListener("pointerleave", () => {
      if (!dragging) hideTooltip();
    });
    markers.appendChild(tip);
  }

  function applySeekMode(): void {
    modeBtn.textContent = advanced ? "Hide advanced" : "Show advanced";
    modeBtn.title = advanced
      ? "Switch to a simple flat seekbar"
      : "Show stacked segments and detail seekbar";
    legend.style.display = advanced ? "" : "none";
    markers.style.display = advanced ? "" : "none";
    stackInner.style.display = advanced ? "" : "none";
    if (!advanced) {
      detailWrap.style.display = "none";
      bar.style.height = "18px";
      bar.style.padding = "0";
      bar.style.overflow = "hidden";
      bar.style.borderRadius = "6px";
      stackInner.replaceChildren();
      markers.innerHTML = "";
      legend.innerHTML = "";
    } else {
      bar.style.padding = "3px 4px 5px";
      bar.style.overflow = "visible";
      bar.style.borderRadius = "8px";
    }
  }

  function paintSegments(): void {
    if (!advanced) {
      stackInner.replaceChildren();
      markers.innerHTML = "";
      legend.innerHTML = "";
      bar.style.height = "18px";
      return;
    }
    stackInner.replaceChildren();
    markers.innerHTML = "";
    legend.innerHTML = "";
    const events = timeline.events;
    const visLen = Math.max(countVisibleEvents(events), 1);
    const calls = buildCallSegments(events);
    const loops = buildLoopSegments(events);
    const spans = allSpans(calls, loops);
    const nEv = events.length;
    const maxD = maxStackDepth(calls, loops, nEv);
    const h = stackHeight(maxD);
    stackInner.style.height = `${h}px`;
    // border-box height includes padding (3+5 vertical)
    bar.style.height = `${h + 8}px`;

    // Full-width depth lanes so empty stack areas still read as layers, not one grey block.
    for (let d = 0; d <= maxD; d++) {
      const lane = document.createElement("div");
      lane.dataset.lane = String(d);
      lane.style.cssText = [
        "position:absolute",
        "left:0",
        "right:0",
        `bottom:${slabBottom(d)}px`,
        `height:${LAYER_H - 1}px`,
        `background:${d % 2 === 0 ? "var(--track, #e2e8f0)" : "var(--track-alt, #f1f5f9)"}`,
        "border-radius:2px",
        "pointer-events:none",
        "z-index:0",
        "box-shadow:0 1px 0 rgba(0,0,0,0.06)",
      ].join(";");
      stackInner.appendChild(lane);
    }

    const active = currentSegment(events, timeline.index);
    const seenNames = new Map<string, string>();

    for (const seg of calls) {
      const endEv = seg.endIndex ?? events.length - 1;
      const v0 = eventIndexToVisual(events, seg.startIndex);
      const v1 = eventIndexToVisual(events, endEv + 1);
      const left = (v0 / visLen) * 100;
      const width = Math.max(((v1 - v0) / visLen) * 100, 0.4);
      const color = COLORS[colorIndexForName(seg.name, COLORS.length)];
      seenNames.set(seg.name, color);
      const depth = callStackDepth(seg, spans, nEv);
      const isActive =
        active?.kind === "call" && active.call_id === seg.call_id;

      stackInner.appendChild(
        makeSlab(left, width, color, depth, {
          seg: `fn-${seg.call_id}`,
          title: `${seg.name} · call ${seg.call_id} · t=${seg.startIndex}…${endEv}`,
          active: Boolean(isActive),
          opacity: 0.88,
          extra: { fn: seg.name },
        }),
      );

      placeStartLabel({
        leftPct: left,
        stackDepth: depth,
        label: shortFnName(seg.name),
        color,
        testid: `seeker-fn-start-${seg.call_id}`,
        title: `Function start · ${seg.name} · t=${seg.startIndex}`,
        seekTo: seg.startIndex + 1,
        eventIndex: seg.startIndex,
      });
    }

    const seenLoops = new Map<number, string>();
    for (const seg of loops) {
      const endEv = seg.endIndex ?? events.length - 1;
      const v0 = eventIndexToVisual(events, seg.startIndex);
      const v1 = eventIndexToVisual(events, endEv + 1);
      const left = (v0 / visLen) * 100;
      const width = Math.max(((v1 - v0) / visLen) * 100, 0.4);
      const color = COLORS[seg.loop_id % COLORS.length];
      seenLoops.set(seg.loop_id, color);
      const depth = loopStackDepth(seg, spans, nEv);
      const isActive =
        active?.kind === "loop" && active.loop_id === seg.loop_id;

      stackInner.appendChild(
        makeSlab(left, width, color, depth, {
          seg: `loop-${seg.loop_id}`,
          title: `loop #${seg.loop_id} · t=${seg.startIndex}…${endEv}`,
          active: Boolean(isActive),
          opacity: 0.92,
          extra: { loopId: String(seg.loop_id) },
        }),
      );

      placeStartLabel({
        leftPct: left,
        stackDepth: depth,
        label: `loop #${seg.loop_id}`,
        color,
        testid: `seeker-loop-start-${seg.loop_id}`,
        title: `Loop start · #${seg.loop_id} · t=${seg.startIndex}`,
        seekTo: seg.startIndex + 1,
        eventIndex: seg.startIndex,
      });

      // One `.` per LoopIter step on this loop band
      let stepN = 0;
      for (let i = seg.startIndex; i <= endEv; i++) {
        const ev = events[i];
        if (ev?.kind !== "LoopIter" || ev.loop_id !== seg.loop_id) continue;
        stepN++;
        const pct = (eventIndexToVisual(events, i) / visLen) * 100;
        const bandH = LAYER_H - 1;
        const dotSize = 4;
        // Vertically centered in the loop band; horizontally centered on the step.
        const bottom =
          slabBottom(depth) + Math.max(0, (bandH - dotSize) / 2);
        const dot = document.createElement("button");
        dot.type = "button";
        dot.dataset.testid = `seeker-loop-step-${seg.loop_id}-${stepN}`;
        dot.dataset.loopStep = String(stepN);
        dot.title = `loop #${seg.loop_id} · step ${stepN} · t=${i}`;
        dot.setAttribute(
          "aria-label",
          `Loop ${seg.loop_id} step ${stepN}`,
        );
        dot.style.cssText = [
          "position:absolute",
          `left:${pct}%`,
          `bottom:${bottom}px`,
          `width:${dotSize}px`,
          `height:${dotSize}px`,
          "transform:translateX(-50%)",
          "pointer-events:auto",
          "cursor:pointer",
          "border:none",
          "padding:0",
          "margin:0",
          "border-radius:50%",
          "background:var(--panel, #fff)",
          "box-shadow:0 0 0 1px var(--shadow, #0006)",
          `z-index:${45 + depth}`,
        ].join(";");
        const seekTo = i + 1;
        dot.addEventListener("click", (e) => {
          e.stopPropagation();
          timeline.seek(seekTo);
        });
        markers.appendChild(dot);
      }
    }

    if (seenNames.size > 0 || seenLoops.size > 0) {
      if (seenNames.size > 0) {
        const title = document.createElement("span");
        title.textContent = "Functions:";
        title.style.color = "var(--muted, #64748b)";
        legend.appendChild(title);
        for (const [name, color] of seenNames) {
          const item = document.createElement("span");
          item.style.cssText = "display:inline-flex;align-items:center;gap:4px;";
          const swatch = document.createElement("span");
          swatch.style.cssText = `width:10px;height:10px;border-radius:2px;background:${color};display:inline-block;`;
          const text = document.createElement("span");
          text.textContent = shortFnName(name);
          text.title = name;
          item.append(swatch, text);
          legend.appendChild(item);
        }
      }
      if (seenLoops.size > 0) {
        const title = document.createElement("span");
        title.textContent = "Loops:";
        title.style.cssText = "color:var(--muted, #64748b);margin-left:8px;";
        legend.appendChild(title);
        for (const [id, color] of seenLoops) {
          const item = document.createElement("span");
          item.style.cssText = "display:inline-flex;align-items:center;gap:4px;";
          const swatch = document.createElement("span");
          swatch.style.cssText = `width:10px;height:10px;border-radius:2px;background:${color};display:inline-block;`;
          const text = document.createElement("span");
          text.textContent = `#${id}`;
          item.append(swatch, text);
          legend.appendChild(item);
        }
      }
    }
  }

  function syncUi(): void {
    const events = timeline.events;
    const visLen = Math.max(countVisibleEvents(events), 1);
    const visIdx = playheadToVisual(events, timeline.index);

    // Scrub / smooth-play own the thumb; don't snap it back to discrete slots.
    if (dragging || detailDragging) {
      indexLabel.textContent = `${visIdx} / ${visLen}`;
      // Skip paintSegments while dragging — avoids hitching the continuous thumb.
      return;
    }
    if (smoothPlaying) {
      // Thumb + detail driven by rAF; throttle segment paint.
      const now = performance.now();
      if (now - lastPaintAt > 80) {
        lastPaintAt = now;
        paintSegments();
      }
      return;
    }

    visPos = visIdx;
    setMainScrubPct((visIdx / visLen) * 100);
    indexLabel.textContent = `${visIdx} / ${visLen}`;
    paintSegments();
    syncDetail();
  }

  function syncDetail(): void {
    if (!advanced) {
      detailWrap.style.display = "none";
      return;
    }
    if (smoothPlaying) {
      syncDetailFromVisPos();
      return;
    }
    const events = timeline.events;
    const seg = currentSegment(events, timeline.index);
    if (!seg) {
      detailWrap.style.display = "none";
      return;
    }
    detailWrap.style.display = "block";
    const color =
      seg.kind === "loop"
        ? COLORS[(seg.loop_id ?? 0) % COLORS.length]
        : COLORS[colorIndexForName(seg.name ?? seg.label, COLORS.length)];
    detailSeg.style.background = color;
    const visLen = Math.max(
      countVisibleInRange(events, seg.startIndex, seg.endIndex),
      1,
    );
    const visIdx = playheadToVisualInRange(
      events,
      timeline.index,
      seg.startIndex,
      seg.endIndex,
    );
    if (!detailDragging) {
      setDetailScrubPct((visIdx / visLen) * 100);
    }
    detailLabel.textContent = `${seg.label} · ${visIdx} / ${visLen}`;
  }

  function detailIndexFromClientX(clientX: number): number | null {
    const events = timeline.events;
    const seg = currentSegment(events, timeline.index);
    if (!seg) return null;
    const ratio = ratioFromClientX(detailBar, clientX);
    const visLen = Math.max(
      countVisibleInRange(events, seg.startIndex, seg.endIndex),
      1,
    );
    return visualToPlayheadInRange(
      events,
      ratio * visLen,
      seg.startIndex,
      seg.endIndex,
    );
  }

  function onDetailPointer(clientX: number, seek: boolean): void {
    stopSmoothPlay();
    const ratio = ratioFromClientX(detailBar, clientX);
    setDetailScrubPct(ratio * 100);
    const t = detailIndexFromClientX(clientX);
    if (t === null) return;
    showTooltipAt(clientX, t);
    if (seek) timeline.seek(t);
  }

  function showTooltipAt(clientX: number, t: number): void {
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
  }

  function hideTooltip(): void {
    tooltip.style.display = "none";
    timeline.setHoverHighlight(null);
  }

  function onPointer(clientX: number, seek: boolean): void {
    const events = timeline.events;
    const visLen = Math.max(countVisibleEvents(events), 1);
    const ratio = ratioFromClientX(bar, clientX);
    // Continuous thumb/progress under the pointer while scrubbing.
    if (seek || dragging) {
      visPos = ratio * visLen;
      setMainScrubPct(ratio * 100);
      syncDetailFromVisPos();
    }
    const t = indexFromClientX(clientX);
    showTooltipAt(clientX, t);
    if (seek) timeline.seek(t);
  }

  bar.addEventListener("pointerdown", (e) => {
    stopSmoothPlay();
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
    // One paint after continuous drag.
    syncUi();
  });
  bar.addEventListener("pointerleave", () => {
    if (!dragging) hideTooltip();
  });

  detailBar.addEventListener("pointerdown", (e) => {
    detailDragging = true;
    detailBar.setPointerCapture(e.pointerId);
    onDetailPointer(e.clientX, true);
  });
  detailBar.addEventListener("pointermove", (e) => {
    if (detailDragging) onDetailPointer(e.clientX, true);
  });
  detailBar.addEventListener("pointerup", (e) => {
    detailDragging = false;
    try {
      detailBar.releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
    syncUi();
  });

  let unsub = timeline.subscribe((ev) => {
    if (ev.type === "tick" || ev.type === "seek") syncUi();
  });

  playBtn.addEventListener("click", () => {
    if (smoothPlaying || timeline.playing) stopSmoothPlay();
    else startSmoothPlay();
  });
  pauseBtn.addEventListener("click", () => stopSmoothPlay());

  modeBtn.addEventListener("click", () => {
    advanced = !advanced;
    applySeekMode();
    syncUi();
  });
  applySeekMode();

  function isTypingTarget(t: EventTarget | null): boolean {
    if (!(t instanceof HTMLElement)) return false;
    const tag = t.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
    if (t.isContentEditable) return true;
    if (t.closest(".cm-editor")) return true;
    return false;
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (isTypingTarget(e.target)) return;
    if (e.code === "Space" || e.key === " ") {
      if (e.repeat) return;
      e.preventDefault();
      if (smoothPlaying || timeline.playing) stopSmoothPlay();
      else startSmoothPlay();
      return;
    }
    if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
      e.preventDefault();
      stopSmoothPlay();
      timeline.step(e.key === "ArrowRight" ? 1 : -1);
    }
  }
  window.addEventListener("keydown", onKeyDown);

  syncUi();

  return {
    update(next) {
      if (next.timeline && next.timeline !== timeline) {
        stopSmoothPlay();
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
      stopSmoothPlay();
      timeline.pause();
      window.removeEventListener("keydown", onKeyDown);
      unsub();
      el.innerHTML = "";
    },
  };
}
