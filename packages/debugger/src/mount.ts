import type { Timeline } from "@rscpp/timeline";
import { playheadEventGroup } from "./eventGroup.js";

export type DebuggerProps = {
  timeline: Timeline;
};

export type MountHandle = {
  update(props: Partial<DebuggerProps>): void;
  destroy(): void;
};

function isTypingTarget(t: EventTarget | null): boolean {
  if (!(t instanceof HTMLElement)) return false;
  if (t.isContentEditable) return true;
  const tag = t.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  return Boolean(t.closest(".cm-editor"));
}

export function mountDebugger(el: HTMLElement, props: DebuggerProps): MountHandle {
  let { timeline } = props;
  let open = false;
  let unsub: (() => void) | null = null;

  el.dataset.testid = "debugger-root";
  el.style.cssText = [
    "position:fixed",
    "top:8px",
    "right:8px",
    "z-index:9999",
    "font:12px/1.4 ui-monospace,SFMono-Regular,Menlo,monospace",
    "display:flex",
    "flex-direction:column",
    "align-items:flex-end",
    "gap:6px",
  ].join(";");

  const btn = document.createElement("button");
  btn.type = "button";
  btn.textContent = "DBG";
  btn.dataset.testid = "debugger-toggle";
  btn.title = "Toggle event debugger (`)";
  btn.style.cssText = [
    "cursor:pointer",
    "border:1px solid var(--control-border, #334155)",
    "background:var(--control-bg, #0f172a)",
    "color:var(--text, #e2e8f0)",
    "border-radius:6px",
    "padding:4px 10px",
    "font:inherit",
    "font-weight:600",
    "letter-spacing:0.04em",
  ].join(";");

  const panel = document.createElement("div");
  panel.dataset.testid = "debugger-panel";
  panel.hidden = true;
  panel.style.cssText = [
    "width:min(420px,calc(100vw - 24px))",
    "max-height:min(60vh,480px)",
    "overflow:auto",
    "background:var(--panel, #0f172a)",
    "color:var(--text, #e2e8f0)",
    "border:1px solid var(--border, #334155)",
    "border-radius:8px",
    "padding:10px",
    "box-shadow:0 8px 24px var(--shadow, #0006)",
  ].join(";");

  const meta = document.createElement("div");
  meta.dataset.testid = "debugger-meta";
  meta.style.cssText =
    "margin-bottom:8px;color:var(--muted, #94a3b8);word-break:break-all;";

  const pre = document.createElement("pre");
  pre.dataset.testid = "debugger-raw";
  pre.style.cssText =
    "margin:0;white-space:pre-wrap;word-break:break-word;color:var(--text, #f8fafc);";

  panel.append(meta, pre);
  el.replaceChildren(btn, panel);

  function setOpen(next: boolean): void {
    open = next;
    panel.hidden = !open;
    btn.style.background = open
      ? "var(--track-alt, #1e293b)"
      : "var(--control-bg, #0f172a)";
    if (open) render();
  }

  function render(): void {
    const i = timeline.index;
    const n = timeline.length;
    const group = playheadEventGroup(timeline.events, i);
    if (group.length === 0) {
      meta.textContent = `0 / ${n} · —`;
      pre.textContent = "(no event yet — playhead at start)";
      return;
    }
    const kinds = group.map((e) => e.kind).join(" · ");
    meta.textContent = `${i} / ${n} · ${kinds}`;
    pre.textContent = JSON.stringify(group, null, 2);
  }

  function bind(): void {
    unsub?.();
    unsub = timeline.subscribe((e) => {
      if (e.type === "tick" || e.type === "seek") {
        if (open) render();
      }
    });
    if (open) render();
  }

  const onBtn = () => setOpen(!open);
  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "`" || e.metaKey || e.ctrlKey || e.altKey) return;
    if (isTypingTarget(e.target)) return;
    e.preventDefault();
    setOpen(!open);
  };

  btn.addEventListener("click", onBtn);
  window.addEventListener("keydown", onKey);
  bind();

  return {
    update(next) {
      if (next.timeline) {
        timeline = next.timeline;
        bind();
      }
    },
    destroy() {
      unsub?.();
      btn.removeEventListener("click", onBtn);
      window.removeEventListener("keydown", onKey);
      el.replaceChildren();
    },
  };
}
