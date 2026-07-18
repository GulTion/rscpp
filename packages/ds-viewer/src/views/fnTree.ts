import { hierarchy, tree as d3tree } from "d3-hierarchy";
import {
  argUnchangedFromParent,
  formatArgValue,
  formatFnLabel,
  type FnTreeNode,
} from "../fnTree.js";

const NS = "http://www.w3.org/2000/svg";
const MAX_NODES = 80;

function ensureStyle(): void {
  if (typeof document === "undefined") return;
  if (document.getElementById("ds-fn-tree-style")) return;
  const style = document.createElement("style");
  style.id = "ds-fn-tree-style";
  style.textContent = `
.ds-fn-card {
  font: 11px/1.35 ui-monospace, monospace;
  padding: 4px 8px;
  border-radius: 6px;
  border: 1px solid #cbd5e1;
  background: #f8fafc;
  color: #0f172a;
  white-space: nowrap;
  box-shadow: 0 1px 2px #0001;
}
.ds-fn-card.active {
  border-color: #0284c7;
  background: #e0f2fe;
  box-shadow: 0 0 0 1px #7dd3fc;
}
.ds-fn-card.done {
  opacity: 0.45;
}
.ds-fn-name { font-weight: 700; }
.ds-fn-arg-same { color: #94a3b8; }
.ds-fn-arg-change { color: #b45309; font-weight: 600; }
`;
  document.head?.appendChild(style);
}

function labelHtml(node: FnTreeNode, parent: FnTreeNode | null): string {
  const base = node.name.includes("::")
    ? node.name.slice(node.name.lastIndexOf("::") + 2)
    : node.name;
  const parts = node.args.map((a) => {
    const same = argUnchangedFromParent(parent, a);
    const cls = same ? "ds-fn-arg-same" : "ds-fn-arg-change";
    // Root / first appearance of a ref: treat object ids as "same-ish" only vs parent
    const showChange = parent ? !same : a.value.kind === "Int" || a.value.kind === "Bool";
    const useCls = parent ? cls : showChange ? "ds-fn-arg-change" : "ds-fn-arg-same";
    return `<span class="${useCls}">${a.name}=${formatArgValue(a.value)}</span>`;
  });
  const args = parts.length ? `(${parts.join(", ")})` : "()";
  return `<span class="ds-fn-name">${base}</span>${args}`;
}

export function renderFnTree(host: HTMLElement, roots: FnTreeNode[]): void {
  host.innerHTML = "";
  ensureStyle();
  if (roots.length === 0) {
    const empty = document.createElement("div");
    empty.style.cssText = "font:12px sans-serif;color:#64748b;padding:4px;";
    empty.textContent = "No function calls yet";
    host.appendChild(empty);
    return;
  }

  // Synthetic root if multiple top-level calls
  const forest: FnTreeNode = {
    call_id: -999,
    parent_id: null,
    name: "",
    args: [],
    enterIndex: -1,
    exitIndex: null,
    active: false,
    children: roots,
  };

  type N = FnTreeNode & { parentRef?: FnTreeNode | null };
  const root = hierarchy(forest as N, (d) => d.children);
  // Attach parent for arg comparison (skip synthetic)
  root.each((d) => {
    if (d.parent && d.parent.data.call_id !== -999) {
      d.data.parentRef = d.parent.data;
    } else if (d.parent?.data.call_id === -999) {
      d.data.parentRef = null;
    }
  });

  const nodes = root.descendants().filter((d) => d.data.call_id !== -999);
  if (nodes.length > MAX_NODES) {
    host.textContent = `function tree too large (${nodes.length} > ${MAX_NODES})`;
    return;
  }

  const layout = d3tree<N>().nodeSize([56, 200]);
  layout(root);

  let minX = Infinity,
    maxX = -Infinity,
    minY = Infinity,
    maxY = -Infinity;
  for (const d of nodes) {
    minX = Math.min(minX, d.x ?? 0);
    maxX = Math.max(maxX, d.x ?? 0);
    minY = Math.min(minY, d.y ?? 0);
    maxY = Math.max(maxY, d.y ?? 0);
  }
  const padX = 16;
  const padY = 20;
  const cardW = 180;
  const w = maxY - minY + padY * 2 + cardW;
  const h = maxX - minX + padX * 2 + 40;

  const svg = document.createElementNS(NS, "svg");
  svg.setAttribute("data-testid", "ds-fn-tree");
  svg.setAttribute("width", String(Math.max(w, 200)));
  svg.setAttribute("height", String(Math.max(h, 80)));

  const g = document.createElementNS(NS, "g");
  // Horizontal tree: swap x/y so depth grows right
  g.setAttribute(
    "transform",
    `translate(${padY - minY},${padX - minX + 20})`,
  );

  for (const d of nodes) {
    if (!d.parent || d.parent.data.call_id === -999) continue;
    const line = document.createElementNS(NS, "path");
    const x0 = d.parent.y ?? 0;
    const y0 = d.parent.x ?? 0;
    const x1 = d.y ?? 0;
    const y1 = d.x ?? 0;
    const mid = (x0 + x1) / 2;
    line.setAttribute("d", `M ${x0} ${y0} C ${mid} ${y0}, ${mid} ${y1}, ${x1} ${y1}`);
    line.setAttribute("fill", "none");
    line.setAttribute("stroke", "#94a3b8");
    line.setAttribute("stroke-width", "1.25");
    g.appendChild(line);
  }

  for (const d of nodes) {
    const x = d.y ?? 0;
    const y = d.x ?? 0;
    const fo = document.createElementNS(NS, "foreignObject");
    fo.setAttribute("x", String(x - 8));
    fo.setAttribute("y", String(y - 16));
    fo.setAttribute("width", "200");
    fo.setAttribute("height", "36");
    const div = document.createElement("div");
    div.className = "ds-fn-card";
    if (d.data.active) div.classList.add("active");
    else div.classList.add("done");
    div.dataset.testid = `ds-fn-node-${d.data.call_id}`;
    div.title = formatFnLabel(d.data);
    div.innerHTML = labelHtml(d.data, d.data.parentRef ?? null);
    fo.appendChild(div);
    g.appendChild(fo);
  }

  svg.appendChild(g);
  host.appendChild(svg);
}
