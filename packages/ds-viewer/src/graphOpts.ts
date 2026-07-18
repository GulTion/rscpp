export type GraphDirection = "directed" | "undirected";

export type GraphViewOpts = {
  direction: GraphDirection;
  /** When true, keep parallel edges; when false, one edge per pair. */
  multigraph: boolean;
};

export type Edge = { from: number; to: number };

export type DrawEdge = {
  from: number;
  to: number;
  /** 0..count-1 among parallel edges sharing the same pair key. */
  index: number;
  count: number;
};

export const DEFAULT_GRAPH_OPTS: GraphViewOpts = {
  direction: "undirected",
  multigraph: false,
};

function pairKey(e: Edge, direction: GraphDirection): string {
  if (direction === "directed") return `${e.from}->${e.to}`;
  const a = Math.min(e.from, e.to);
  const b = Math.max(e.from, e.to);
  return `${a}--${b}`;
}

/**
 * Normalize raw directed edges for display.
 * Undirected: collapse u–v / v–u to one undirected pair.
 * Non-multigraph: at most one edge per pair key.
 * Multigraph: keep multiplicity; assign index for parallel curves.
 */
export function normalizeEdges(edges: Edge[], opts: GraphViewOpts): DrawEdge[] {
  const buckets = new Map<string, Edge[]>();
  for (const e of edges) {
    if (e.from === e.to && opts.direction === "undirected") {
      // keep self-loops as a single undirected loop key
    }
    const key = pairKey(e, opts.direction);
    let list = buckets.get(key);
    if (!list) {
      list = [];
      buckets.set(key, list);
    }
    if (!opts.multigraph && list.length > 0) continue;
    // For undirected, store canonical from=min to=max for drawing consistency
    if (opts.direction === "undirected") {
      list.push({
        from: Math.min(e.from, e.to),
        to: Math.max(e.from, e.to),
      });
    } else {
      list.push(e);
    }
  }
  const out: DrawEdge[] = [];
  for (const list of buckets.values()) {
    const count = list.length;
    list.forEach((e, index) => {
      out.push({ from: e.from, to: e.to, index, count });
    });
  }
  return out;
}
