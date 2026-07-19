import type { HeapSnapshot, ObjectState, ValueJson } from "@rscpp/timeline";
import { formatVal } from "../diff.js";
import type { GraphEncoding } from "../represent.js";
import type { AccessHighlight } from "../access.js";
import {
  EMPTY_ACCESS,
  attachIndexHint,
  ensureAccessStyle,
  fillAccessCell,
  markCellAccess,
  markIndexAccess,
  shouldAnimateWrite,
} from "../access.js";

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

function headerStyle(): string {
  return "border:1px solid var(--border, #cbd5e1);padding:2px 6px;background:var(--track-alt, #f1f5f9);color:var(--muted, #64748b);font:11px ui-monospace,monospace;text-align:center;";
}

function cellStyle(): string {
  return "border:1px solid var(--border, #ccc);padding:2px 6px;text-align:center;overflow:hidden;vertical-align:middle;color:var(--text, inherit);";
}

export function renderMatrix(
  host: HTMLElement,
  obj: ObjectState,
  heap: HeapSnapshot,
  access: AccessHighlight = EMPTY_ACCESS,
  _prev?: ObjectState,
): void {
  host.innerHTML = "";
  ensureAccessStyle();
  const table = document.createElement("table");
  table.dataset.testid = "ds-matrix";
  table.style.cssText = "border-collapse:collapse;";

  const rows = obj.elems ?? [];
  let cols = 0;
  for (const rowVal of rows) {
    if (rowVal.kind === "Object") {
      cols = Math.max(cols, heap.objects.get(rowVal.value)?.elems?.length ?? 0);
    } else {
      cols = Math.max(cols, 1);
    }
  }

  const head = document.createElement("tr");
  const corner = document.createElement("th");
  corner.style.cssText = headerStyle();
  corner.textContent = "";
  head.appendChild(corner);
  for (let j = 0; j < cols; j++) {
    const th = document.createElement("th");
    th.style.cssText = headerStyle();
    th.textContent = String(j);
    th.dataset.testid = `ds-matrix-col-${j}`;
    head.appendChild(th);
  }
  table.appendChild(head);

  rows.forEach((rowVal, i) => {
    const tr = document.createElement("tr");
    const rowTh = document.createElement("th");
    rowTh.style.cssText = headerStyle();
    rowTh.textContent = String(i);
    rowTh.dataset.testid = `ds-matrix-row-${i}`;
    tr.appendChild(rowTh);

    if (rowVal.kind === "Object") {
      const row = heap.objects.get(rowVal.value);
      for (let j = 0; j < cols; j++) {
        const td = document.createElement("td");
        td.style.cssText = cellStyle();
        td.dataset.testid = `ds-matrix-${i}-${j}`;
        const nextV = row?.elems?.[j];
        const key = `${i},${j}`;
        const anim = shouldAnimateWrite(access, { i, j });
        const oldText = access.writeOld.get(key);
        fillAccessCell(td, formatVal(nextV), oldText, anim);
        markCellAccess(td, { i, j }, access);
        attachIndexHint(td, access, { i, j });
        tr.appendChild(td);
      }
    } else {
      const td = document.createElement("td");
      td.style.cssText = cellStyle();
      td.colSpan = Math.max(cols, 1);
      const anim = shouldAnimateWrite(access, i);
      fillAccessCell(td, formatVal(rowVal), access.writeOld.get(String(i)), anim);
      markIndexAccess(td, i, access);
      attachIndexHint(td, access, i);
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
  weighted = false,
): { from: number; to: number; weight?: number }[] {
  const edges: { from: number; to: number; weight?: number }[] = [];

  if (encoding === "adjacency-matrix") {
    const elems = obj.elems ?? [];
    elems.forEach((rowVal, i) => {
      if (rowVal.kind !== "Object") return;
      const row = heap.objects.get(rowVal.value);
      row?.elems?.forEach((cell, j) => {
        if (cell.kind !== "Int" || cell.value === 0) return;
        edges.push(
          weighted
            ? { from: i, to: j, weight: cell.value }
            : { from: i, to: j },
        );
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
        if (cell.kind === "Int") {
          edges.push({ from: i, to: cell.value });
          continue;
        }
        // weighted: neighbor is pair/object {to, weight}
        if (weighted && cell.kind === "Object") {
          const pair = heap.objects.get(cell.value);
          const pe = pair?.elems ?? [];
          if (
            pe.length >= 2 &&
            pe[0].kind === "Int" &&
            pe[1].kind === "Int"
          ) {
            edges.push({ from: i, to: pe[0].value, weight: pe[1].value });
          }
        }
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
          if (weighted && n.kind === "Object") {
            const pair = heap.objects.get(n.value);
            const pe = pair?.elems ?? [];
            if (
              pe.length >= 2 &&
              pe[0].kind === "Int" &&
              pe[1].kind === "Int"
            ) {
              edges.push({ from, to: pe[0].value, weight: pe[1].value });
            }
          }
        }
      }
    }
    return edges;
  }

  // edge-list: [u,v] or weighted [u,v,w]
  for (const e of obj.elems ?? []) {
    if (e.kind !== "Object") continue;
    const child = heap.objects.get(e.value);
    const ce = child?.elems ?? [];
    if (ce.length >= 2 && ce[0].kind === "Int" && ce[1].kind === "Int") {
      if (weighted && ce.length >= 3 && ce[2].kind === "Int") {
        edges.push({
          from: ce[0].value,
          to: ce[1].value,
          weight: ce[2].value,
        });
      } else {
        edges.push({ from: ce[0].value, to: ce[1].value });
      }
    }
  }
  return edges;
}
