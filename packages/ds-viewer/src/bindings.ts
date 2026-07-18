import type { HeapSnapshot, ObjectState, ValueJson } from "@rscpp/timeline";

/** Resolve a local value to a live heap object id, if any. */
export function objectIdOf(value: ValueJson): number | null {
  if (value.kind === "Object" && typeof value.value === "number") {
    return value.value;
  }
  if (value.kind === "Ref" || value.kind === "Ptr") {
    const addr = value.value as { kind?: string; value?: unknown } | null;
    if (addr && addr.kind === "Heap" && typeof addr.value === "number") {
      return addr.value;
    }
  }
  return null;
}

export type VarBinding = {
  id: number;
  /** Variable name(s) bound to this object (deduped). */
  title: string;
  obj: ObjectState;
};

/**
 * Live heap objects that are named by at least one local in any call frame.
 * Title is the variable name (comma-joined if several aliases).
 */
export function bindingsFromSnapshot(snap: HeapSnapshot): VarBinding[] {
  const namesById = new Map<number, Set<string>>();
  for (const frame of snap.frames) {
    for (const [name, value] of frame.locals) {
      if (!name) continue;
      const id = objectIdOf(value);
      if (id === null || !snap.objects.has(id)) continue;
      let set = namesById.get(id);
      if (!set) {
        set = new Set();
        namesById.set(id, set);
      }
      set.add(name);
    }
  }
  return [...namesById.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([id, names]) => ({
      id,
      title: [...names].join(", "),
      obj: snap.objects.get(id)!,
    }));
}
