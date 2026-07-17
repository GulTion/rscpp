import type { ObjectState } from "@rscpp/timeline";
import { diffElems, formatVal } from "../diff.js";

export function renderLinear(
  host: HTMLElement,
  prev: ObjectState | undefined,
  next: ObjectState | undefined,
  mode: "array" | "table" | "stack" | "queue",
): void {
  host.innerHTML = "";
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
      if (d.kind === "add" || d.kind === "change") tdV.className = "ds-flash";
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
    if (d.kind === "add" || d.kind === "change") cell.className = "ds-flash";
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

const style = document.createElement("style");
style.textContent = `
.ds-flash { animation: dsflash 0.45s ease; background: #fef08a; }
@keyframes dsflash { from { background: #facc15; } to { background: transparent; } }
`;
if (typeof document !== "undefined" && !document.getElementById("ds-viewer-style")) {
  style.id = "ds-viewer-style";
  document.head?.appendChild(style);
}
