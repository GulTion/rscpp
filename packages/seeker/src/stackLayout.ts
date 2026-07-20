import type { CallSegment, LoopSegment } from "./segments.js";

/** Pixel height of one depth layer in the stacked seekbar. */
export const LAYER_H = 12;

/** Extra pad above the top slab. */
export const STACK_PAD = 4;

export type StackSpan = {
  startIndex: number;
  endIndex: number | null;
  /** Stable id for tests / debugging. */
  key: string;
};

export function stackHeight(maxDepth: number): number {
  return (Math.max(maxDepth, 0) + 1) * LAYER_H + STACK_PAD;
}

/** Bottom offset (px from stack bottom) for a slab at `stackDepth`. */
export function slabBottom(stackDepth: number): number {
  return stackDepth * LAYER_H;
}

function spanEnd(seg: StackSpan, eventCount: number): number {
  return seg.endIndex ?? Math.max(eventCount - 1, seg.startIndex);
}

/** True if `outer` fully wraps `inner` (and is not the same span). */
export function spansContain(
  outer: StackSpan,
  inner: StackSpan,
  eventCount: number,
): boolean {
  if (outer.key === inner.key) return false;
  const o0 = outer.startIndex;
  const o1 = spanEnd(outer, eventCount);
  const i0 = inner.startIndex;
  const i1 = spanEnd(inner, eventCount);
  if (o0 > i0 || o1 < i1) return false;
  // Must be strictly larger in at least one endpoint (avoid ties).
  return o0 < i0 || o1 > i1;
}

/**
 * Wedding-cake depth: how many other call/loop spans fully contain this one.
 * Sibling activations (e.g. dfs roots for each component) share a depth;
 * a loop between parent dfs and child dfs sits between them.
 */
export function containmentDepth(
  seg: StackSpan,
  all: StackSpan[],
  eventCount: number,
): number {
  let n = 0;
  for (const other of all) {
    if (spansContain(other, seg, eventCount)) n++;
  }
  return n;
}

export function callSpan(seg: CallSegment): StackSpan {
  return {
    startIndex: seg.startIndex,
    endIndex: seg.endIndex,
    key: `fn-${seg.call_id}`,
  };
}

export function loopSpan(seg: LoopSegment): StackSpan {
  return {
    startIndex: seg.startIndex,
    endIndex: seg.endIndex,
    key: `loop-${seg.loop_id}`,
  };
}

export function allSpans(
  calls: CallSegment[],
  loops: LoopSegment[],
): StackSpan[] {
  return [...calls.map(callSpan), ...loops.map(loopSpan)];
}

export function callStackDepth(
  seg: CallSegment,
  all: StackSpan[],
  eventCount: number,
): number {
  return containmentDepth(callSpan(seg), all, eventCount);
}

export function loopStackDepth(
  seg: LoopSegment,
  all: StackSpan[],
  eventCount: number,
): number {
  return containmentDepth(loopSpan(seg), all, eventCount);
}

export function maxStackDepth(
  calls: CallSegment[],
  loops: LoopSegment[],
  eventCount: number,
): number {
  const all = allSpans(calls, loops);
  let max = 0;
  for (const c of calls) max = Math.max(max, callStackDepth(c, all, eventCount));
  for (const l of loops) max = Math.max(max, loopStackDepth(l, all, eventCount));
  return max;
}
