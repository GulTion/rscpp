import { hierarchy, tree as d3tree } from "d3-hierarchy";
import {
  argUnchangedFromParent,
  formatArgValue,
  formatFnLabel,
  type FnTreeNode,
} from "../fnTree.js";

const NS = "http://www.w3.org/2000/svg";
const MAX_NODES = 80;
const R = 16;

function ensureStyle(): void {
  if (typeof document === "undefined") return;
  if (document.getElementById("ds-fn-tree-style")) return;
  const style = document.createElement("style");
  style.id = "ds-fn-tree-style";
  style.textContent = `
.ds-fn-node-circle { fill: #e2e8f0; stroke: #64748b; stroke-width: 1.5px; }
.ds-fn-node-circle.active { fill: #7dd3fc; stroke: #0369a1; stroke-width: 2.5px; }
.ds-fn-node-circle.done { fill: #f1f5f9; stroke: #94a3b8; opacity: 0.55; }
.ds-fn-edge { fill: none; stroke: #94a3b8; stroke-width: 1.5px; }
.ds-fn-edge.active { stroke: #0284c7; stroke-width: 2px; }
.ds-fn-label-name { font: 700 10px ui-sans-serif, system-ui, sans-serif; fill: #0f172a; }
.ds-fn-label-args { font: 9px ui-monospace, monospace; fill: #475569; }
.ds-fn-arg-same { fill: #94a3b8; }
.ds-fn-arg-change { fill: #b45309; font-weight: 700; }
`;
  document.head?.appendChild(style);
}

function shortName(name: string): string {
  const i = name.lastIndexOf("::");
  return i >= 0 ? name.slice(i + 2) : name;
}

/** Compact changing-arg summary for under the node, e.g. `u=1`. */
function changingArgsLine(node: FnTreeNode, parent: FnTreeNode | null): string {
  const parts: string[] = [];
  for (const a of node.args) {
    const same = parent ? argUnchangedFromParent(parent, a) : false;
    if (!same) parts.push(`${a.name}=${formatArgValue(a.value)}`);
  }
  // If everything is "same" (or root with only refs), still show primary scalar if any
  if (parts.length === 0) {
    const scalar = node.args.find(
      (a) => a.value.kind === "Int" || a.value.kind === "Bool",
    );
    if (scalar) return `${scalar.name}=${formatArgValue(scalar.value)}`;
  }
  return parts.slice(0, 3).join(" ");
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
  root.each((d) => {
    if (d.parent && d.parent.data.call_id !== -999) {
      d.data.parentRef = d.parent.data;
    } else {
      d.data.parentRef = null;
    }
  });

  const nodes = root.descendants().filter((d) => d.data.call_id !== -999);
  if (nodes.length > MAX_NODES) {
    host.textContent = `function tree too large (${nodes.length} > ${MAX_NODES})`;
    return;
  }

  // Top-down tree: sibling spacing × depth spacing
  const layout = d3tree<N>().nodeSize([72, 70]);
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
  const pad = 36;
  const labelExtra = 28;
  const w = maxX - minX + pad * 2;
  const h = maxY - minY + pad * 2 + labelExtra;

  const svg = document.createElementNS(NS, "svg");
  svg.setAttribute("data-testid", "ds-fn-tree");
  svg.setAttribute("width", String(Math.max(w, 160)));
  svg.setAttribute("height", String(Math.max(h, 100)));

  const g = document.createElementNS(NS, "g");
  g.setAttribute("transform", `translate(${pad - minX},${pad - minY})`);

  // Edges first (under nodes)
  for (const d of nodes) {
    if (!d.parent || d.parent.data.call_id === -999) continue;
    const x0 = d.parent.x ?? 0;
    const y0 = d.parent.y ?? 0;
    const x1 = d.x ?? 0;
    const y1 = d.y ?? 0;
    const midY = (y0 + y1) / 2;
    const path = document.createElementNS(NS, "path");
    // Orthogonal-ish curve from parent circle bottom to child top
    path.setAttribute(
      "d",
      `M ${x0} ${y0 + R} C ${x0} ${midY}, ${x1} ${midY}, ${x1} ${y1 - R}`,
    );
    path.setAttribute("class", "ds-fn-edge");
    if (d.data.active && d.parent.data.active) path.classList.add("active");
    g.appendChild(path);
  }

  for (const d of nodes) {
    const x = d.x ?? 0;
    const y = d.y ?? 0;
    const parent = d.data.parentRef ?? null;

    const circle = document.createElementNS(NS, "circle");
    circle.setAttribute("cx", String(x));
    circle.setAttribute("cy", String(y));
    circle.setAttribute("r", String(R));
    circle.setAttribute("class", "ds-fn-node-circle");
    if (d.data.active) circle.classList.add("active");
    else circle.classList.add("done");
    circle.setAttribute("data-testid", `ds-fn-node-${d.data.call_id}`);
    circle.setAttribute("title", formatFnLabel(d.data));

    // Abbreviation inside circle
    const abbr = document.createElementNS(NS, "text");
    abbr.setAttribute("x", String(x));
    abbr.setAttribute("y", String(y + 3.5));
    abbr.setAttribute("text-anchor", "middle");
    abbr.setAttribute("class", "ds-fn-label-name");
    abbr.setAttribute("font-size", "9");
    const sn = shortName(d.data.name);
    abbr.textContent = sn.length > 5 ? sn.slice(0, 4) + "…" : sn;

    // Name + changing args under node
    const nameT = document.createElementNS(NS, "text");
    nameT.setAttribute("x", String(x));
    nameT.setAttribute("y", String(y + R + 12));
    nameT.setAttribute("text-anchor", "middle");
    nameT.setAttribute("class", "ds-fn-label-name");
    nameT.textContent = sn;

    const argsLine = changingArgsLine(d.data, parent);
    const argsT = document.createElementNS(NS, "text");
    argsT.setAttribute("x", String(x));
    argsT.setAttribute("y", String(y + R + 23));
    argsT.setAttribute("text-anchor", "middle");
    argsT.setAttribute("class", "ds-fn-label-args");
    if (argsLine) {
      const changeNames = new Set(
        d.data.args
          .filter((a) => !(parent && argUnchangedFromParent(parent, a)))
          .map((a) => a.name),
      );
      const bits = argsLine.split(" ");
      bits.forEach((bit, i) => {
        if (i > 0) {
          const sp = document.createElementNS(NS, "tspan");
          sp.textContent = " ";
          argsT.appendChild(sp);
        }
        const name = bit.split("=")[0] ?? "";
        const span = document.createElementNS(NS, "tspan");
        span.textContent = bit;
        span.setAttribute(
          "class",
          changeNames.has(name) ? "ds-fn-arg-change" : "ds-fn-arg-same",
        );
        argsT.appendChild(span);
      });
    }

    const title = document.createElementNS(NS, "title");
    title.textContent = formatFnLabel(d.data);
    circle.appendChild(title);

    g.append(circle, abbr, nameT, argsT);
  }

  svg.appendChild(g);
  host.appendChild(svg);
}
