import type { HeapSnapshot, ObjectState, ValueJson } from "@rscpp/timeline";
import { formatVal } from "../diff.js";
import type { GraphEncoding } from "../represent.js";
import type { AccessHighlight } from "../access.js";
import { EMPTY_ACCESS, ensureAccessStyle } from "../access.js";

export function renderRaw(host: HTMLElement, obj: ObjectState | undefined): void {
  host.innerHTML = "";
  const pre = document.createElement("pre");
  pre.dataset.testid = "ds-raw";
  pre.style.cssText = "font:12px monospace;white-space:pre-wrap;";
  pre.textContent = JSON.stringify(
    obj
      ? {
          type_name: obj.type_name,
          elems: obj.elems,
          entries: obj.entries,
        }
      : null,
    null,
    2,
  );
  host.appendChild(pre);
}

export function renderMatrix(
  host: HTMLElement,
  obj: ObjectState,
  heap: HeapSnapshot,
  access: AccessHighlight = EMPTY_ACCESS,
): void {
  host.innerHTML = "";
  ensureAccessStyle();
  const table = document.createElement("table");
  table.dataset.testid = "ds-matrix";
  table.style.borderCollapse = "collapse";
  const cur = new Set(access.currentCells.map((c) => `${c.i},${c.j}`));
  const trail = new Set(access.trailCells.map((c) => `${c.i},${c.j}`));
  const curRow = new Set(access.current);
  const trailRow = new Set(access.trail);

  (obj.elems ?? []).forEach((rowVal, i) => {
    const tr = document.createElement("tr");
    const rowOnlyCurrent =
      curRow.has(i) && ![...cur].some((k) => k.startsWith(`${i},`));
    const rowOnlyTrail =
      trailRow.has(i) &&
      !rowOnlyCurrent &&
      ![...trail].some((k) => k.startsWith(`${i},`)) &&
      ![...cur].some((k) => k.startsWith(`${i},`));

    if (rowVal.kind === "Object") {
      const row = heap.objects.get(rowVal.value);
      (row?.elems ?? []).forEach((cell, j) => {
        const td = document.createElement("td");
        td.textContent = formatVal(cell);
        td.style.cssText = "border:1px solid #ccc;padding:2px 6px;";
        td.dataset.testid = `ds-matrix-${i}-${j}`;
        const k = `${i},${j}`;
        if (cur.has(k) || rowOnlyCurrent) td.classList.add("ds-access-current");
        else if (trail.has(k) || rowOnlyTrail) td.classList.add("ds-access-trail");
        tr.appendChild(td);
      });
    } else {
      const td = document.createElement("td");
      td.textContent = formatVal(rowVal);
      td.style.cssText = "border:1px solid #ccc;padding:2px 6px;";
      if (curRow.has(i)) td.classList.add("ds-access-current");
      else if (trailRow.has(i)) td.classList.add("ds-access-trail");
      tr.appendChild(td);
    }
    table.appendChild(tr);
  });
  host.appendChild(table);
}

export function edgesFromObject(
  obj: ObjectState,
  heap: HeapSnapshot,
  encoding: GraphEncoding,
): { from: number; to: number }[] {
  const edges: { from: number; to: number }[] = [];

  if (encoding === "adjacency-matrix") {
    const elems = obj.elems ?? [];
    elems.forEach((rowVal, i) => {
      if (rowVal.kind !== "Object") return;
      const row = heap.objects.get(rowVal.value);
      row?.elems?.forEach((cell, j) => {
        if (cell.kind === "Int" && cell.value !== 0) edges.push({ from: i, to: j });
      });
    });
    return edges;
  }

  if (encoding === "adjacency-list") {
    const elems = obj.elems ?? [];
    elems.forEach((rowVal, i) => {
      if (rowVal.kind !== "Object") return;
      const row = heap.objects.get(rowVal.value);
      for (const cell of row?.elems ?? []) {
        if (cell.kind === "Int") edges.push({ from: i, to: cell.value });
      }
    });
    for (const ent of obj.entries ?? []) {
      const key = ent.key as ValueJson;
      const from =
        typeof key === "object" &&
        key &&
        "kind" in key &&
        (key as ValueJson).kind === "Int"
          ? ((key as { value: number }).value as number)
          : Number(key);
      const val = ent.value;
      if (val?.kind === "Object") {
        const neigh = heap.objects.get(val.value);
        for (const n of neigh?.elems ?? []) {
          if (n.kind === "Int") edges.push({ from, to: n.value });
        }
      }
    }
    return edges;
  }

  // edge-list
  for (const e of obj.elems ?? []) {
    if (e.kind !== "Object") continue;
    const child = heap.objects.get(e.value);
    const ce = child?.elems ?? [];
    if (ce.length >= 2 && ce[0].kind === "Int" && ce[1].kind === "Int") {
      edges.push({ from: ce[0].value, to: ce[1].value });
    }
  }
  return edges;
}
