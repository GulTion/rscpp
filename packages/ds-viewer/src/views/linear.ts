import type { ObjectState } from "@rscpp/timeline";
import { diffElems, formatVal } from "../diff.js";
import type { AccessHighlight } from "../access.js";
import {
  EMPTY_ACCESS,
  attachIndexHint,
  ensureAccessStyle,
  fillAccessCell,
  markIndexAccess,
  shouldAnimateWrite,
} from "../access.js";

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
      const anim = shouldAnimateWrite(access, d.index);
      const oldText = access.writeOld.get(String(d.index));
      fillAccessCell(tdV, formatVal(d.value), oldText, anim);
      tdI.style.cssText = tdV.style.cssText =
        "border:1px solid var(--border, #ccc);padding:2px 6px;overflow:hidden;text-align:center;color:var(--text, inherit);";
      markIndexAccess(tdV, d.index, access);
      markIndexAccess(tr, d.index, access);
      attachIndexHint(tdV, access, d.index);
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
    if (d.kind === "remove") {
      // Show pop target briefly when process is pop.
      if (access.structural?.op === "pop" && access.structural.index === d.index) {
        const cell = document.createElement("div");
        cell.dataset.testid = `ds-cell-${d.index}`;
        cell.style.cssText =
          "border:1px solid var(--border, #94a3b8);padding:4px 8px;border-radius:4px;min-width:2ch;text-align:center;overflow:hidden;color:var(--text, inherit);";
        cell.textContent = formatVal(d.value);
        markIndexAccess(cell, d.index, access);
        wrap.appendChild(cell);
      }
      continue;
    }
    const cell = document.createElement("div");
    cell.dataset.testid = `ds-cell-${d.index}`;
    cell.style.cssText =
      "border:1px solid var(--border, #94a3b8);padding:4px 8px;border-radius:4px;min-width:2ch;text-align:center;overflow:hidden;color:var(--text, inherit);";
    const anim = shouldAnimateWrite(access, d.index);
    const oldText = access.writeOld.get(String(d.index));
    fillAccessCell(cell, formatVal(d.value), oldText, anim);
    markIndexAccess(cell, d.index, access);
    attachIndexHint(cell, access, d.index);
    wrap.appendChild(cell);
  }
  if (mode === "stack") {
    const label = document.createElement("div");
    label.textContent = "↑ top";
    label.style.cssText = "font:11px monospace;color:var(--muted, #64748b);";
    wrap.prepend(label);
  }
  if (mode === "queue") {
    const label = document.createElement("div");
    label.textContent = "front →";
    label.style.cssText = "font:11px monospace;color:var(--muted, #64748b);width:100%;";
    wrap.prepend(label);
  }
  host.appendChild(wrap);
}
