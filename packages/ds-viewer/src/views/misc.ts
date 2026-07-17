import type { HeapSnapshot, ObjectState, ValueJson } from "@rscpp/timeline";
import { formatVal } from "../diff.js";

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
): void {
  host.innerHTML = "";
  const table = document.createElement("table");
  table.dataset.testid = "ds-matrix";
  table.style.borderCollapse = "collapse";
  for (const rowVal of obj.elems ?? []) {
    const tr = document.createElement("tr");
    if (rowVal.kind === "Object") {
      const row = heap.objects.get(rowVal.value);
      for (const cell of row?.elems ?? []) {
        const td = document.createElement("td");
        td.textContent = formatVal(cell);
        td.style.cssText = "border:1px solid #ccc;padding:2px 6px;";
        tr.appendChild(td);
      }
    } else {
      const td = document.createElement("td");
      td.textContent = formatVal(rowVal);
      tr.appendChild(td);
    }
    table.appendChild(tr);
  }
  host.appendChild(table);
}

export function edgesFromObject(
  obj: ObjectState,
  heap: HeapSnapshot,
): { from: number; to: number }[] {
  const edges: { from: number; to: number }[] = [];
  const elems = obj.elems ?? [];
  // adjacency matrix of ints
  if (elems.every((e) => e.kind === "Object")) {
    elems.forEach((rowVal, i) => {
      if (rowVal.kind !== "Object") return;
      const row = heap.objects.get(rowVal.value);
      row?.elems?.forEach((cell, j) => {
        if (cell.kind === "Int" && cell.value !== 0) edges.push({ from: i, to: j });
      });
    });
    return edges;
  }
  // edge list: vector of pair objects or vector<vector<int>> length-2
  for (const e of elems) {
    if (e.kind !== "Object") continue;
    const child = heap.objects.get(e.value);
    const ce = child?.elems ?? [];
    if (ce.length >= 2 && ce[0].kind === "Int" && ce[1].kind === "Int") {
      edges.push({ from: ce[0].value, to: ce[1].value });
    }
  }
  // map adjacency
  for (const ent of obj.entries ?? []) {
    const key = ent.key as ValueJson;
    const from =
      typeof key === "object" && key && "kind" in key && (key as ValueJson).kind === "Int"
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
