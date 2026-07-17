import type { Timeline } from "@rscpp/timeline";
import {
  buildCallSegments,
  buildLoopSegments,
  colorIndexForName,
} from "./segments.js";

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

export function mountSeeker(el: HTMLElement, props: SeekerProps): MountHandle {
  let { timeline, source } = props;
  el.dataset.testid = "seeker-root";
  el.innerHTML = "";

  // Single scrubber bar: segment colors + progress + thumb + hover tooltip
  const barWrap = document.createElement("div");
  barWrap.style.cssText = "position:relative;margin:8px 0 4px;padding-top:28px;";
  barWrap.dataset.testid = "seeker-scrubber";

  const markers = document.createElement("div");
  markers.style.cssText =
    "position:absolute;left:0;right:0;top:0;height:26px;pointer-events:none;z-index:4;";
  markers.dataset.testid = "seeker-loop-markers";

  const bar = document.createElement("div");
  bar.style.cssText =
    "position:relative;height:22px;background:#e2e8f0;border-radius:7px;cursor:pointer;overflow:hidden;user-select:none;";
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
    "bottom:calc(100% - 4px)",
    "left:0",
    "transform:translateX(-50%)",
    "display:none",
    "z-index:6",
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
  barWrap.append(tooltip, markers, bar);

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
  const legend = document.createElement("div");
  legend.dataset.testid = "seeker-fn-legend";
  legend.style.cssText =
    "display:flex;flex-wrap:wrap;gap:6px 10px;align-items:center;font:11px ui-sans-serif,system-ui,sans-serif;color:#334155;margin-top:6px;";
  el.append(barWrap, controls, legend);

  let dragging = false;

  function indexFromClientX(clientX: number): number {
    const rect = bar.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (clientX - rect.left) / Math.max(rect.width, 1)));
    return Math.round(ratio * timeline.length);
  }

  function paintSegments(): void {
    bar.querySelectorAll("[data-seg]").forEach((n) => n.remove());
    markers.innerHTML = "";
    legend.innerHTML = "";
    const len = Math.max(timeline.length, 1);

    // Nested function call segments (color by name; child inset 1px per depth)
    const calls = buildCallSegments(timeline.events);
    const seenNames = new Map<string, string>();
    for (const seg of calls) {
      const end = seg.endIndex ?? timeline.length;
      const left = (seg.startIndex / len) * 100;
      const width = Math.max(((end - seg.startIndex + 1) / len) * 100, 0.4);
      const color = COLORS[colorIndexForName(seg.name, COLORS.length)];
      seenNames.set(seg.name, color);

      // depth 0 = full height; each nested level +1px top/bottom padding inside parent
      const inset = seg.depth; // px
      const segEl = document.createElement("div");
      segEl.dataset.seg = `fn-${seg.call_id}`;
      segEl.dataset.fn = seg.name;
      segEl.dataset.depth = String(seg.depth);
      segEl.title = `${seg.name} · call ${seg.call_id} · t=${seg.startIndex}…${end}`;
      segEl.style.cssText = [
        "position:absolute",
        `left:${left}%`,
        `width:${width}%`,
        `top:${inset}px`,
        `bottom:${inset}px`,
        `background:${color}`,
        "opacity:0.85",
        `z-index:${seg.depth}`,
        "pointer-events:none",
        "box-sizing:border-box",
        // 1px visual pad so parent color shows as a border around the child
        seg.depth > 0 ? "outline:1px solid transparent" : "",
      ]
        .filter(Boolean)
        .join(";");
      bar.insertBefore(segEl, progress);

      // Static marker: function started
      const tip = document.createElement("button");
      tip.type = "button";
      tip.dataset.testid = `seeker-fn-start-${seg.call_id}`;
      tip.title = `Function started · ${seg.name} · t=${seg.startIndex}`;
      tip.setAttribute("aria-label", `Function started: ${seg.name}`);
      tip.style.cssText = [
        "position:absolute",
        `left:${left}%`,
        "bottom:0",
        "transform:translateX(-50%)",
        "pointer-events:auto",
        "cursor:pointer",
        "border:none",
        "padding:0",
        "background:transparent",
        "display:flex",
        "flex-direction:column",
        "align-items:center",
        "gap:1px",
        `z-index:${10 + seg.depth}`,
      ].join(";");

      const label = document.createElement("span");
      label.textContent = shortFnName(seg.name);
      label.style.cssText = [
        "font:9px/1 ui-sans-serif,system-ui,sans-serif",
        "font-weight:600",
        "padding:2px 5px",
        "border-radius:4px",
        `background:${color}`,
        "color:#fff",
        "white-space:nowrap",
        "box-shadow:0 1px 2px #0003",
        "max-width:72px",
        "overflow:hidden",
        "text-overflow:ellipsis",
      ].join(";");

      const tick = document.createElement("span");
      tick.style.cssText = [
        "width:0",
        "height:0",
        "border-left:4px solid transparent",
        "border-right:4px solid transparent",
        `border-top:5px solid ${color}`,
      ].join(";");

      tip.append(label, tick);
      tip.addEventListener("click", (e) => {
        e.stopPropagation();
        timeline.seek(seg.startIndex);
      });
      tip.addEventListener("pointerenter", (e) => {
        e.stopPropagation();
        const rect = tip.getBoundingClientRect();
        const wrapRect = barWrap.getBoundingClientRect();
        tooltip.style.left = `${rect.left + rect.width / 2 - wrapRect.left}px`;
        tooltip.style.display = "block";
        tooltip.textContent = `Function started · ${seg.name} · t=${seg.startIndex}`;
        const ev = timeline.events[seg.startIndex];
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

    // Nested loop segments (same nesting + 1px inset model as functions).
    // Inset starts at 1px so parent function color remains visible at the edges.
    const loops = buildLoopSegments(timeline.events);
    const seenLoops = new Map<number, string>();
    for (const seg of loops) {
      const end = seg.endIndex ?? timeline.length;
      const left = (seg.startIndex / len) * 100;
      const width = Math.max(((end - seg.startIndex + 1) / len) * 100, 0.4);
      const color = COLORS[seg.loop_id % COLORS.length];
      seenLoops.set(seg.loop_id, color);
      // +1 so even outermost loop leaves 1px of function color above/below
      const inset = seg.depth + 1;
      const segEl = document.createElement("div");
      segEl.dataset.seg = `loop-${seg.loop_id}`;
      segEl.dataset.loopId = String(seg.loop_id);
      segEl.dataset.depth = String(seg.depth);
      segEl.title = `loop #${seg.loop_id} · t=${seg.startIndex}…${end}`;
      segEl.style.cssText = [
        "position:absolute",
        `left:${left}%`,
        `width:${width}%`,
        `top:${inset}px`,
        `bottom:${inset}px`,
        `background:${color}`,
        "opacity:0.9",
        `z-index:${20 + seg.depth}`,
        "pointer-events:none",
        "box-sizing:border-box",
      ].join(";");
      bar.insertBefore(segEl, progress);
    }

    // Loop-start pins
    for (const seg of loops) {
      const left = (seg.startIndex / len) * 100;
      const color = COLORS[seg.loop_id % COLORS.length];
      const tip = document.createElement("button");
      tip.type = "button";
      tip.dataset.testid = `seeker-loop-start-${seg.loop_id}-${seg.startIndex}`;
      tip.title = `Loop started · id ${seg.loop_id} · t=${seg.startIndex}`;
      tip.setAttribute("aria-label", `Loop started, id ${seg.loop_id}`);
      tip.style.cssText = [
        "position:absolute",
        `left:${left}%`,
        "top:0",
        "transform:translateX(-50%)",
        "pointer-events:auto",
        "cursor:pointer",
        "border:none",
        "padding:0",
        "background:transparent",
        "display:flex",
        "flex-direction:column",
        "align-items:center",
        "gap:1px",
        "z-index:40",
      ].join(";");

      const label = document.createElement("span");
      label.textContent = "loop start";
      label.style.cssText = [
        "font:8px/1 ui-sans-serif,system-ui,sans-serif",
        "font-weight:700",
        "padding:2px 5px",
        "border-radius:3px",
        `background:${color}`,
        "color:#fff",
        "box-shadow:0 1px 2px #0003",
        "white-space:nowrap",
      ].join(";");

      const tick = document.createElement("span");
      tick.style.cssText = [
        "width:0",
        "height:0",
        "border-left:4px solid transparent",
        "border-right:4px solid transparent",
        `border-top:5px solid ${color}`,
      ].join(";");

      tip.append(label, tick);
      tip.addEventListener("click", (e) => {
        e.stopPropagation();
        timeline.seek(seg.startIndex);
      });
      tip.addEventListener("pointerenter", (e) => {
        e.stopPropagation();
        const rect = tip.getBoundingClientRect();
        const wrapRect = barWrap.getBoundingClientRect();
        tooltip.style.left = `${rect.left + rect.width / 2 - wrapRect.left}px`;
        tooltip.style.display = "block";
        tooltip.textContent = `Loop started · id ${seg.loop_id} · t=${seg.startIndex}`;
        const ev = timeline.events[seg.startIndex];
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

    // Legend: functions + loops
    if (seenNames.size > 0 || seenLoops.size > 0) {
      if (seenNames.size > 0) {
        const title = document.createElement("span");
        title.textContent = "Functions:";
        title.style.color = "#64748b";
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
        title.style.cssText = "color:#64748b;margin-left:8px;";
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
