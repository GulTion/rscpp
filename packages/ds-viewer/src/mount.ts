import type { ObjectState, Timeline } from "@rscpp/timeline";
import { proposeRepresentations, type Representation } from "./represent.js";
import { renderLinear } from "./views/linear.js";
import { renderMatrix, renderRaw } from "./views/misc.js";
import { renderGraph, renderTree } from "./views/graph.js";

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
    case "graph":
    case "edge-list":
    case "adjacency":
      renderGraph(host, obj, snap);
      break;
    default:
      renderRaw(host, obj);
  }
}

export function mountDsViewer(el: HTMLElement, props: DsViewerProps): MountHandle {
  let timeline = props.timeline;
  let objId = props.objId;
  let mode: "all" | "single" = props.mode ?? "all";
  const pref = new Map<number, Representation>();
  if (objId !== null && props.representation) pref.set(objId, props.representation);
  let onRepresentationChange = props.onRepresentationChange;
  const prevById = new Map<number, ObjectState>();

  el.dataset.testid = "ds-root";
  el.innerHTML = "";

  const toolbar = document.createElement("div");
  toolbar.style.cssText =
    "display:flex;gap:8px;align-items:center;margin-bottom:8px;flex-wrap:wrap;";

  const modeSel = document.createElement("select");
  modeSel.dataset.testid = "ds-mode";
  for (const [v, label] of [
    ["all", "All live Allocs"],
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

  const body = document.createElement("div");
  body.dataset.testid = "ds-body";
  body.style.cssText =
    "display:flex;flex-direction:column;gap:12px;overflow:auto;max-height:100%;";

  toolbar.append(modeSel, select);
  el.append(toolbar, body);

  function reprFor(id: number, obj: ObjectState): Representation {
    const proposed = proposeRepresentations(obj, timeline.snapshot());
    if (pref.has(id)) {
      const p = pref.get(id)!;
      if (proposed.includes(p)) return p;
    }
    return proposed[0] ?? "raw";
  }

  function paintPane(
    pane: HTMLElement,
    id: number,
    obj: ObjectState,
    snap: ReturnType<Timeline["snapshot"]>,
  ): void {
    pane.innerHTML = "";
    pane.dataset.testid = `ds-pane-${id}`;
    pane.style.cssText =
      "border:1px solid #cbd5e1;border-radius:6px;padding:8px;background:#f8fafc;";

    const head = document.createElement("div");
    head.style.cssText =
      "display:flex;justify-content:space-between;align-items:center;gap:8px;margin-bottom:6px;";
    const title = document.createElement("strong");
    title.style.font = "12px ui-monospace, monospace";
    title.textContent = `#${id} ${obj.type_name}`;
    title.dataset.testid = `ds-pane-title-${id}`;

    const localSel = document.createElement("select");
    localSel.dataset.testid = `ds-repr-${id}`;
    const options = proposeRepresentations(obj, snap);
    const repr = reprFor(id, obj);
    for (const r of options) {
      const o = document.createElement("option");
      o.value = r;
      o.textContent = r;
      if (r === repr) o.selected = true;
      localSel.appendChild(o);
    }
    localSel.addEventListener("change", () => {
      pref.set(id, localSel.value as Representation);
      onRepresentationChange?.(localSel.value as Representation);
      paint();
    });

    head.append(title, localSel);
    const viewHost = document.createElement("div");
    pane.append(head, viewHost);
    renderOne(viewHost, obj, prevById.get(id), snap, repr);
    prevById.set(id, structuredClone(obj));
  }

  function paint(): void {
    const snap = timeline.snapshot();
    body.innerHTML = "";
    select.style.display = mode === "single" ? "" : "none";

    if (mode === "all") {
      const ids = [...snap.objects.keys()].sort((a, b) => a - b);
      if (ids.length === 0) {
        body.textContent = "No live heap objects at this playhead";
        return;
      }
      for (const id of ids) {
        const obj = snap.objects.get(id)!;
        const pane = document.createElement("div");
        body.appendChild(pane);
        paintPane(pane, id, obj, snap);
      }
      return;
    }

    // single
    select.innerHTML = "";
    const obj = objId !== null ? snap.objects.get(objId) : undefined;
    if (!obj || objId === null) {
      body.textContent =
        objId === null ? "Select an object" : `Object #${objId} not in snapshot`;
      return;
    }
    const options = proposeRepresentations(obj, snap);
    const repr = reprFor(objId, obj);
    for (const r of options) {
      const o = document.createElement("option");
      o.value = r;
      o.textContent = r;
      if (r === repr) o.selected = true;
      select.appendChild(o);
    }
    const pane = document.createElement("div");
    body.appendChild(pane);
    paintPane(pane, objId, obj, snap);
  }

  modeSel.addEventListener("change", () => {
    mode = modeSel.value as "all" | "single";
    paint();
  });

  select.addEventListener("change", () => {
    const r = select.value as Representation;
    if (objId !== null) pref.set(objId, r);
    onRepresentationChange?.(r);
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
        unsub = timeline.subscribe((ev) => {
          if (ev.type === "tick" || ev.type === "seek") paint();
        });
      }
      if (next.objId !== undefined) objId = next.objId;
      if (next.mode !== undefined) {
        mode = next.mode;
        modeSel.value = mode;
      }
      if (next.representation && objId !== null) pref.set(objId, next.representation);
      if (next.onRepresentationChange) onRepresentationChange = next.onRepresentationChange;
      paint();
    },
    destroy() {
      unsub();
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
