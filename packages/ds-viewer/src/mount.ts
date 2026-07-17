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
  objId: number | null;
  representation?: Representation;
  onRepresentationChange?: (r: Representation) => void;
};

export function mountDsViewer(el: HTMLElement, props: DsViewerProps): MountHandle {
  let timeline = props.timeline;
  let objId = props.objId;
  const pref = new Map<number, Representation>();
  if (objId !== null && props.representation) pref.set(objId, props.representation);
  let onRepresentationChange = props.onRepresentationChange;

  el.dataset.testid = "ds-root";
  el.innerHTML = "";

  const toolbar = document.createElement("div");
  toolbar.style.cssText = "display:flex;gap:8px;align-items:center;margin-bottom:8px;";
  const select = document.createElement("select");
  select.dataset.testid = "ds-repr-select";
  const body = document.createElement("div");
  body.dataset.testid = "ds-body";
  el.append(toolbar, body);
  toolbar.appendChild(select);

  let prevObj: ObjectState | undefined;

  function currentRepr(obj: ObjectState): Representation {
    const proposed = proposeRepresentations(obj, timeline.snapshot());
    if (objId !== null && pref.has(objId)) {
      const p = pref.get(objId)!;
      if (proposed.includes(p)) return p;
    }
    return proposed[0] ?? "raw";
  }

  function paint(): void {
    const snap = timeline.snapshot();
    const obj = objId !== null ? snap.objects.get(objId) : undefined;
    select.innerHTML = "";
    if (!obj) {
      body.textContent = objId === null ? "Select an object" : `Object #${objId} not in snapshot`;
      prevObj = undefined;
      return;
    }
    const options = proposeRepresentations(obj, snap);
    const repr = currentRepr(obj);
    for (const r of options) {
      const o = document.createElement("option");
      o.value = r;
      o.textContent = r;
      if (r === repr) o.selected = true;
      select.appendChild(o);
    }

    switch (repr) {
      case "array":
      case "table":
      case "stack":
      case "queue":
        renderLinear(body, prevObj, obj, repr);
        break;
      case "matrix":
        renderMatrix(body, obj, snap);
        break;
      case "tree":
        renderTree(body, obj, snap);
        break;
      case "graph":
      case "edge-list":
      case "adjacency":
        renderGraph(body, obj, snap);
        break;
      default:
        renderRaw(body, obj);
    }
    prevObj = structuredClone(obj);
  }

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
        unsub = timeline.subscribe((ev) => {
          if (ev.type === "tick" || ev.type === "seek") paint();
        });
      }
      if (next.objId !== undefined) {
        objId = next.objId;
        prevObj = undefined;
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
