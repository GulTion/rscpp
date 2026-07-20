/**
 * Events that still advance reconstruct, but don't get a seeker/play landing.
 *
 * Pairing rule (any DS): when runtime emits two events for one store, silence the
 * twin reconstruct ignores (or that only duplicates work):
 *   - VarAssign + Write Local     → silence VarAssign  (prefer Write)
 *   - Write MapEntry/Index + ContainerMod → silence Write (prefer ContainerMod)
 * Write Local / Global stays visible (chips + highlight).
 *
 * LoopIter: only the first LoopIter for a loop_id (until LoopEnd) is visible;
 * later iterations of the same instance are silent.
 */
export const UI_SILENT_KINDS = new Set([
  "ScopeEnter",
  "ScopeExit",
  "Branch",
  "RefBind",
  "Alloc",
  "Dealloc",
  "Step",
  "VarAssign",
  "VarDestroy",
]);

/** Write slots that are paired with ContainerMod — silence the Write half. */
const SILENT_WRITE_SLOTS = new Set(["MapEntry", "Index"]);

/** ContainerMod ops that are redundant twins (e.g. before map_assign). */
const SILENT_CONTAINER_OPS = new Set(["map_default_insert"]);

export type SilentEvent = {
  kind: string;
  slot?: { kind?: string } | null;
  op?: string | null;
  loop_id?: number | null;
};

export function isUiSilentKind(kind: string | undefined): boolean {
  return kind !== undefined && UI_SILENT_KINDS.has(kind);
}

/** Whether `events[index]` should be skipped by seeker/play. */
export function isUiSilentAt(events: SilentEvent[], index: number): boolean {
  const ev = events[index];
  if (!ev) return false;
  if (isUiSilentKind(ev.kind)) return true;
  if (ev.kind === "Write" && SILENT_WRITE_SLOTS.has(String(ev.slot?.kind ?? ""))) {
    return true;
  }
  if (
    ev.kind === "ContainerMod" &&
    SILENT_CONTAINER_OPS.has(String(ev.op ?? ""))
  ) {
    return true;
  }
  if (ev.kind === "LoopIter") {
    const id = ev.loop_id;
    for (let j = index - 1; j >= 0; j--) {
      const prev = events[j];
      if (prev.kind === "LoopEnd" && prev.loop_id === id) return false;
      if (prev.kind === "LoopIter" && prev.loop_id === id) return true;
    }
    return false;
  }
  return false;
}

/** @deprecated Prefer `isUiSilentAt(events, i)` when loop context matters. */
export function isUiSilentEvent(
  ev: SilentEvent | undefined,
  events?: SilentEvent[],
  index?: number,
): boolean {
  if (events !== undefined && index !== undefined) {
    return isUiSilentAt(events, index);
  }
  if (!ev) return false;
  if (isUiSilentKind(ev.kind)) return true;
  if (ev.kind === "Write" && SILENT_WRITE_SLOTS.has(String(ev.slot?.kind ?? ""))) {
    return true;
  }
  if (
    ev.kind === "ContainerMod" &&
    SILENT_CONTAINER_OPS.has(String(ev.op ?? ""))
  ) {
    return true;
  }
  // Without context, first LoopIter is treated as visible
  return false;
}

/**
 * After applying `events[0..index)`, the "current" event is `events[index-1]`.
 * If that event is UI-silent, snap to the next (or previous) non-silent playhead.
 * Reconstruction still includes every silent event up to the snapped index.
 */
export function snapPlayheadIndex(events: SilentEvent[], index: number): number {
  const n = events.length;
  let i = Math.max(0, Math.min(Math.floor(index), n));
  if (i === 0) return 0;
  if (!isUiSilentAt(events, i - 1)) return i;

  for (let j = i + 1; j <= n; j++) {
    if (j === 0 || !isUiSilentAt(events, j - 1)) return j;
  }
  for (let j = i - 1; j >= 0; j--) {
    if (j === 0 || !isUiSilentAt(events, j - 1)) return j;
  }
  return i;
}
