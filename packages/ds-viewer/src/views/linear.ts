import type { ObjectState } from "@rscpp/timeline";
import { diffElems, formatVal } from "../diff.js";
import type { AccessHighlight } from "../access.js";
import { EMPTY_ACCESS, ensureAccessStyle } from "../access.js";

function markAccess(el: HTMLElement, index: number, access: AccessHighlight): void {
  if (access.current.includes(index)) el.classList.add("ds-access-current");
  else if (access.trail.includes(index)) el.classList.add("ds-access-trail");
}

export function renderLinear(
  host: HTMLElement,
  prev: ObjectState | undefined,
  next: ObjectState | undefined,
  mode: "array" | "table" | "stack" | "queue",
  access: AccessHighlight = EMPTY_ACCESS,
): void {
  host.innerHTML = "";
  ensureAccessStyle();
  const wrap = document.createElement("div");
  wrap.dataset.testid = "ds-linear";
  wrap.style.cssText =
    mode === "stack" || mode === "queue"
      ? "display:flex;flex-direction:column;gap:4px;align-items:stretch;"
      : "display:flex;flex-wrap:wrap;gap:4px;";

  if (mode === "table") {
    const table = document.createElement("table");
    table.style.borderCollapse = "collapse";
    const diffs = diffElems(prev, next);
    for (const d of diffs) {
      if (d.kind === "remove") continue;
      const tr = document.createElement("tr");
      const tdI = document.createElement("td");
      const tdV = document.createElement("td");
      tdI.textContent = String(d.index);
      tdV.textContent = formatVal(d.value);
      tdI.style.cssText = tdV.style.cssText = "border:1px solid #ccc;padding:2px 6px;";
      if (d.kind === "add" || d.kind === "change") tdV.classList.add("ds-flash");
      markAccess(tdV, d.index, access);
      markAccess(tr, d.index, access);
      tr.dataset.testid = `ds-row-${d.index}`;
      tr.append(tdI, tdV);
      table.appendChild(tr);
    }
    host.appendChild(table);
    return;
  }

  const diffs = diffElems(prev, next);
  const items = mode === "stack" ? [...diffs].reverse() : diffs;
  for (const d of items) {
    if (d.kind === "remove") continue;
    const cell = document.createElement("div");
    cell.dataset.testid = `ds-cell-${d.index}`;
    cell.textContent = formatVal(d.value);
    cell.style.cssText =
      "border:1px solid #94a3b8;padding:4px 8px;border-radius:4px;min-width:2ch;text-align:center;";
    if (d.kind === "add" || d.kind === "change") cell.classList.add("ds-flash");
    markAccess(cell, d.index, access);
    wrap.appendChild(cell);
  }
  if (mode === "stack") {
    const label = document.createElement("div");
    label.textContent = "↑ top";
    label.style.cssText = "font:11px monospace;color:#64748b;";
    wrap.prepend(label);
  }
  if (mode === "queue") {
    const label = document.createElement("div");
    label.textContent = "front →";
    label.style.cssText = "font:11px monospace;color:#64748b;width:100%;";
    wrap.prepend(label);
  }
  host.appendChild(wrap);
}
