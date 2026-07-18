import type { ObjectState, Timeline } from "@rscpp/timeline";
import {
  GRAPH_ENCODINGS,
  isGraphEncoding,
  normalizeRepresentation,
  proposeRepresentations,
  representationLabel,
  type Representation,
} from "./represent.js";
import { renderLinear } from "./views/linear.js";
import { renderMatrix, renderRaw } from "./views/misc.js";
import { renderGraph, renderTree } from "./views/graph.js";
import {
  applyPos,
  autoPack,
  canvasExtent,
  type Pos,
  type Rect,
} from "./layout.js";
import { bindingsFromSnapshot } from "./bindings.js";
import {
  DEFAULT_GRAPH_OPTS,
  type GraphViewOpts,
} from "./graphOpts.js";

export type MountHandle = {
  update(props: Partial<DsViewerProps>): void;
  destroy(): void;
};

export type DsViewerProps = {
  timeline: Timeline;
  /** Single object focus; ignored when `mode === "all"`. */
  objId: number | null;
  /** `all` = every live Alloc in the current snapshot (default). */
  mode?: "all" | "single";
  representation?: Representation;
  onRepresentationChange?: (r: Representation) => void;
};

function renderOne(
  host: HTMLElement,
  obj: ObjectState,
  prev: ObjectState | undefined,
  snap: ReturnType<Timeline["snapshot"]>,
  repr: Representation,
  graphOpts: GraphViewOpts,
): void {
  switch (repr) {
    case "array":
    case "table":
    case "stack":
    case "queue":
      renderLinear(host, prev, obj, repr);
      break;
    case "matrix":
      renderMatrix(host, obj, snap);
      break;
    case "tree":
      renderTree(host, obj, snap);
      break;
    case "adjacency-list":
    case "adjacency-matrix":
    case "edge-list":
      renderGraph(host, obj, snap, repr, graphOpts);
      break;
    default:
      renderRaw(host, obj);
  }
}

/** Fill a select with flat options + Graph optgroup for encodings. */
function fillReprSelect(
  selectEl: HTMLSelectElement,
  options: Representation[],
  selected: Representation,
): void {
  selectEl.innerHTML = "";
  const graphs = options.filter(isGraphEncoding);
  const other = options.filter((r) => !isGraphEncoding(r));
  for (const r of other) {
    const o = document.createElement("option");
    o.value = r;
    o.textContent = representationLabel(r);
    if (r === selected) o.selected = true;
    selectEl.appendChild(o);
  }
  if (graphs.length > 0) {
    const group = document.createElement("optgroup");
    group.label = "Graph";
    for (const r of GRAPH_ENCODINGS) {
      if (!graphs.includes(r)) continue;
      const o = document.createElement("option");
      o.value = r;
      o.textContent = representationLabel(r);
      if (r === selected) o.selected = true;
      group.appendChild(o);
    }
    selectEl.appendChild(group);
  }
}

export function mountDsViewer(el: HTMLElement, props: DsViewerProps): MountHandle {
  let timeline = props.timeline;
  let objId = props.objId;
  let mode: "all" | "single" = props.mode ?? "all";
  const pref = new Map<number, Representation>();
  if (objId !== null && props.representation) {
    const n = normalizeRepresentation(props.representation);
    if (n) pref.set(objId, n);
  }
  let onRepresentationChange = props.onRepresentationChange;
  const prevById = new Map<number, ObjectState>();
  /** Sticky layout by Alloc id. */
  const positions = new Map<number, Pos>();
  const graphOptsById = new Map<number, GraphViewOpts>();
  let zTop = 1;

  function optsFor(id: number): GraphViewOpts {
    return graphOptsById.get(id) ?? { ...DEFAULT_GRAPH_OPTS };
  }

  el.dataset.testid = "ds-root";
  el.innerHTML = "";
  el.style.cssText = "display:flex;flex-direction:column;min-height:0;height:100%;";

  const toolbar = document.createElement("div");
  toolbar.style.cssText =
    "display:flex;gap:8px;align-items:center;margin-bottom:6px;flex-wrap:wrap;flex-shrink:0;";

  const modeSel = document.createElement("select");
  modeSel.dataset.testid = "ds-mode";
  for (const [v, label] of [
    ["all", "Named variables"],
    ["single", "Single object"],
  ] as const) {
    const o = document.createElement("option");
    o.value = v;
    o.textContent = label;
    if (v === mode) o.selected = true;
    modeSel.appendChild(o);
  }

  const select = document.createElement("select");
  select.dataset.testid = "ds-repr-select";
  select.title = "Default / focused representation";

  const resetBtn = document.createElement("button");
  resetBtn.type = "button";
  resetBtn.textContent = "Reset layout";
  resetBtn.dataset.testid = "ds-reset-layout";
  resetBtn.title = "Clear saved positions and re-pack";

  const canvas = document.createElement("div");
  canvas.dataset.testid = "ds-canvas";
  canvas.style.cssText = [
    "position:relative",
    "flex:1",
    "min-height:120px",
    "overflow:auto",
    "background:#f1f5f9",
    "border:1px solid #e2e8f0",
    "border-radius:4px",
  ].join(";");

  toolbar.append(modeSel, select, resetBtn);
  el.append(toolbar, canvas);

  function reprFor(id: number, obj: ObjectState): Representation {
    const proposed = proposeRepresentations(obj, timeline.snapshot());
    if (pref.has(id)) {
      const raw = pref.get(id)!;
      const p = normalizeRepresentation(raw) ?? raw;
      if (proposed.includes(p)) return p;
    }
    return proposed[0] ?? "raw";
  }

  function bindDrag(handle: HTMLElement, pane: HTMLElement, id: number): void {
    handle.style.cursor = "grab";
    handle.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      // Don't start drag from form controls in the header.
      const t = e.target as HTMLElement;
      if (t.closest("select,button,input,a")) return;
      e.preventDefault();
      handle.setPointerCapture(e.pointerId);
      handle.style.cursor = "grabbing";
      zTop += 1;
      pane.style.zIndex = String(zTop);
      const startX = e.clientX;
      const startY = e.clientY;
      const orig = positions.get(id) ?? {
        x: pane.offsetLeft,
        y: pane.offsetTop,
      };
      const onMove = (ev: PointerEvent) => {
        const next = {
          x: Math.max(0, orig.x + (ev.clientX - startX)),
          y: Math.max(0, orig.y + (ev.clientY - startY)),
        };
        positions.set(id, next);
        applyPos(pane, next);
        growCanvas();
      };
      const onUp = (ev: PointerEvent) => {
        handle.releasePointerCapture(ev.pointerId);
        handle.style.cursor = "grab";
        handle.removeEventListener("pointermove", onMove);
        handle.removeEventListener("pointerup", onUp);
        handle.removeEventListener("pointercancel", onUp);
      };
      handle.addEventListener("pointermove", onMove);
      handle.addEventListener("pointerup", onUp);
      handle.addEventListener("pointercancel", onUp);
    });
  }

  function growCanvas(): void {
    const rects: Rect[] = [];
    canvas.querySelectorAll<HTMLElement>("[data-pane]").forEach((p) => {
      rects.push({
        x: p.offsetLeft,
        y: p.offsetTop,
        w: p.offsetWidth,
        h: p.offsetHeight,
      });
    });
    const { w, h } = canvasExtent(
      rects,
      canvas.clientWidth || 280,
      canvas.clientHeight || 120,
    );
    canvas.style.minWidth = `${w}px`;
    canvas.style.minHeight = `${h}px`;
  }

  function paintPane(
    id: number,
    titleText: string,
    obj: ObjectState,
    snap: ReturnType<Timeline["snapshot"]>,
    occupied: Rect[],
  ): HTMLElement {
    const pane = document.createElement("div");
    pane.dataset.pane = String(id);
    pane.dataset.testid = `ds-pane-${id}`;
    pane.style.cssText = [
      "position:absolute",
      "width:max-content",
      "max-width:min(480px,100%)",
      "border:1px solid #cbd5e1",
      "border-radius:4px",
      "padding:2px",
      "background:#fff",
      "box-shadow:0 1px 2px #0001",
      "z-index:1",
    ].join(";");

    const head = document.createElement("div");
    head.dataset.testid = `ds-pane-handle-${id}`;
    head.style.cssText =
      "display:flex;justify-content:space-between;align-items:center;gap:4px;padding:1px 2px;user-select:none;";
    const title = document.createElement("strong");
    title.style.font = "11px ui-monospace, monospace";
    title.textContent = titleText;
    title.title = `${titleText} · ${obj.type_name} #${id}`;
    title.dataset.testid = `ds-pane-title-${id}`;

    const localSel = document.createElement("select");
    localSel.dataset.testid = `ds-repr-${id}`;
    localSel.style.cssText = "font:11px sans-serif;max-width:9rem;";
    const options = proposeRepresentations(obj, snap);
    const repr = reprFor(id, obj);
    fillReprSelect(localSel, options, repr);
    localSel.addEventListener("change", () => {
      const next =
        normalizeRepresentation(localSel.value) ?? (localSel.value as Representation);
      pref.set(id, next);
      onRepresentationChange?.(next);
      paint();
    });

    head.append(title, localSel);

    const gOpts = optsFor(id);
    if (isGraphEncoding(repr)) {
      const graphBar = document.createElement("div");
      graphBar.style.cssText =
        "display:flex;gap:6px;align-items:center;padding:1px 2px;font:11px sans-serif;";
      graphBar.dataset.testid = `ds-graph-opts-${id}`;

      const dirSel = document.createElement("select");
      dirSel.dataset.testid = `ds-graph-dir-${id}`;
      for (const [v, label] of [
        ["undirected", "Undirected"],
        ["directed", "Directed"],
      ] as const) {
        const o = document.createElement("option");
        o.value = v;
        o.textContent = label;
        if (v === gOpts.direction) o.selected = true;
        dirSel.appendChild(o);
      }
      dirSel.addEventListener("change", () => {
        graphOptsById.set(id, {
          ...optsFor(id),
          direction: dirSel.value as GraphViewOpts["direction"],
        });
        paint();
      });

      const multiLab = document.createElement("label");
      multiLab.style.cssText = "display:inline-flex;gap:3px;align-items:center;";
      const multi = document.createElement("input");
      multi.type = "checkbox";
      multi.checked = gOpts.multigraph;
      multi.dataset.testid = `ds-graph-multi-${id}`;
      multi.addEventListener("change", () => {
        graphOptsById.set(id, { ...optsFor(id), multigraph: multi.checked });
        paint();
      });
      multiLab.append(multi, document.createTextNode("Multigraph"));

      graphBar.append(dirSel, multiLab);
      pane.append(head, graphBar);
    } else {
      pane.append(head);
    }

    const viewHost = document.createElement("div");
    viewHost.style.cssText = "overflow:auto;max-width:100%;";
    pane.append(viewHost);
    canvas.appendChild(pane);
    renderOne(viewHost, obj, prevById.get(id), snap, repr, gOpts);
    prevById.set(id, structuredClone(obj));

    let pos = positions.get(id);
    if (!pos) {
      const w = Math.max(pane.offsetWidth, 40);
      const h = Math.max(pane.offsetHeight, 24);
      pos = autoPack(w, h, occupied);
      positions.set(id, pos);
    }
    applyPos(pane, pos);
    occupied.push({
      x: pos.x,
      y: pos.y,
      w: pane.offsetWidth,
      h: pane.offsetHeight,
    });
    bindDrag(head, pane, id);
    return pane;
  }

  function paint(): void {
    const snap = timeline.snapshot();
    canvas.innerHTML = "";
    select.style.display = mode === "single" ? "" : "none";
    const occupied: Rect[] = [];

    if (mode === "all") {
      const bindings = bindingsFromSnapshot(snap);
      if (bindings.length === 0) {
        const empty = document.createElement("div");
        empty.style.cssText = "padding:8px;font:12px sans-serif;color:#64748b;";
        empty.textContent = "No variable-bound structures at this playhead";
        canvas.appendChild(empty);
        return;
      }
      for (const b of bindings) {
        paintPane(b.id, b.title, b.obj, snap, occupied);
      }
      growCanvas();
      return;
    }

    select.innerHTML = "";
    const obj = objId !== null ? snap.objects.get(objId) : undefined;
    if (!obj || objId === null) {
      const empty = document.createElement("div");
      empty.style.cssText = "padding:8px;font:12px sans-serif;color:#64748b;";
      empty.textContent =
        objId === null ? "Select an object" : `Object #${objId} not in snapshot`;
      canvas.appendChild(empty);
      return;
    }
    const options = proposeRepresentations(obj, snap);
    const repr = reprFor(objId, obj);
    fillReprSelect(select, options, repr);
    const bound = bindingsFromSnapshot(snap).find((b) => b.id === objId);
    paintPane(objId, bound?.title ?? String(objId), obj, snap, occupied);
    growCanvas();
  }

  modeSel.addEventListener("change", () => {
    mode = modeSel.value as "all" | "single";
    paint();
  });

  select.addEventListener("change", () => {
    const r =
      normalizeRepresentation(select.value) ?? (select.value as Representation);
    if (objId !== null) pref.set(objId, r);
    onRepresentationChange?.(r);
    paint();
  });

  resetBtn.addEventListener("click", () => {
    positions.clear();
    paint();
  });

  let unsub = timeline.subscribe((ev) => {
    if (ev.type === "tick" || ev.type === "seek") paint();
  });
  paint();

  return {
    update(next) {
      if (next.timeline && next.timeline !== timeline) {
        unsub();
        timeline = next.timeline;
        prevById.clear();
        positions.clear();
        graphOptsById.clear();
        unsub = timeline.subscribe((ev) => {
          if (ev.type === "tick" || ev.type === "seek") paint();
        });
      }
      if (next.objId !== undefined) objId = next.objId;
      if (next.mode !== undefined) {
        mode = next.mode;
        modeSel.value = mode;
      }
      if (next.representation && objId !== null) {
        const n = normalizeRepresentation(next.representation);
        if (n) pref.set(objId, n);
      }
      if (next.onRepresentationChange) onRepresentationChange = next.onRepresentationChange;
      paint();
    },
    destroy() {
      unsub();
      positions.clear();
      el.innerHTML = "";
    },
  };
}

/** Collect Alloc ids from timeline events for an object picker. */
export function listAllocIds(timeline: Timeline): { id: number; type_name: string }[] {
  const out: { id: number; type_name: string }[] = [];
  const seen = new Set<number>();
  for (const ev of timeline.events) {
    if (ev.kind === "Alloc" && typeof ev.id === "number" && !seen.has(ev.id)) {
      seen.add(ev.id);
      out.push({ id: ev.id, type_name: String(ev.type_name ?? "object") });
    }
  }
  return out;
}

/** Live object ids in the current snapshot. */
export function listLiveIds(timeline: Timeline): number[] {
  return [...timeline.snapshot().objects.keys()].sort((a, b) => a - b);
}
