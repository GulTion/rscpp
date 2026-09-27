import type { HeapSnapshot, ValueJson } from "@rscpp/timeline";

const MAX_ELEMS = 8;
const MAX_DEPTH = 2;

function formatScalar(value: ValueJson): string {
  switch (value.kind) {
    case "Int":
    case "Float":
    case "Bool":
    case "Char":
    case "Str":
      return String(value.value);
    case "Void":
      return "void";
    case "Nullptr":
      return "nullptr";
    default:
      return value.kind;
  }
}

function formatElems(
  elems: ValueJson[],
  heap: HeapSnapshot | undefined,
  depth: number,
): string {
  const parts: string[] = [];
  const n = Math.min(elems.length, MAX_ELEMS);
  for (let i = 0; i < n; i++) {
    parts.push(formatInner(elems[i], heap, depth));
  }
  if (elems.length > MAX_ELEMS) parts.push("…");
  return parts.join(",");
}

function formatInner(
  value: ValueJson,
  heap: HeapSnapshot | undefined,
  depth: number,
): string {
  if (depth > MAX_DEPTH) return "…";
  if (value.kind === "Object") {
    const obj = heap?.objects.get(value.value);
    if (!obj) return `#${value.value}`;
    if (obj.elems) return `[${formatElems(obj.elems, heap, depth + 1)}]`;
    if (obj.entries?.length) return `{…${obj.entries.length}}`;
    return `#${value.value}`;
  }
  if (value.kind === "Ref" || value.kind === "Ptr") {
    return formatAddress(value.value, heap, depth);
  }
  return formatScalar(value);
}

function formatAddress(
  addr: unknown,
  heap: HeapSnapshot | undefined,
  depth: number,
): string {
  if (!addr || typeof addr !== "object") return "…";
  const a = addr as { kind?: string; value?: unknown };
  if (a.kind === "Heap" && typeof a.value === "number") {
    return formatInner({ kind: "Object", value: a.value }, heap, depth);
  }
  if (a.kind === "Null") return "nullptr";
  return "…";
}

/** True for scalars / easy values — not containers (vector, map, …). */
export function isSimpleChipValue(
  value: ValueJson | { kind: string; value?: unknown },
  heap?: HeapSnapshot,
): boolean {
  const v = value as ValueJson;
  switch (v.kind) {
    case "Int":
    case "Float":
    case "Bool":
    case "Char":
    case "Str":
    case "Void":
    case "Nullptr":
      return true;
    case "Object": {
      const obj = heap?.objects.get(v.value);
      if (!obj) return true; // bare id
      if (obj.type_name === "closure" || obj.type_name === "functor") return false;
      if (obj.elems && obj.elems.length > 0) return false;
      if (obj.entries && obj.entries.length > 0) return false;
      return true;
    }
    case "Ref":
    case "Ptr": {
      const addr = v.value as { kind?: string; value?: unknown } | null;
      if (!addr || typeof addr !== "object") return false;
      if (addr.kind === "Heap" && typeof addr.value === "number") {
        return isSimpleChipValue({ kind: "Object", value: addr.value }, heap);
      }
      if (addr.kind === "Null") return true;
      return false;
    }
    default:
      return false;
  }
}

/** Compact chip text; resolves Object/Ref to heap contents when snapshot is provided. */
export function formatChip(
  value: ValueJson | { kind: string; value?: unknown },
  heap?: HeapSnapshot,
): string {
  const v = value as ValueJson;
  switch (v.kind) {
    case "Int":
    case "Float":
    case "Bool":
    case "Char":
    case "Str":
      return String(v.value);
    case "Void":
      return "void";
    case "Nullptr":
      return "nullptr";
    case "Object": {
      const obj = heap?.objects.get(v.value);
      if (!obj) return `#${v.value}`;
      if (obj.type_name === "closure" || obj.type_name === "functor") return "";
      if (obj.elems) return `[${formatElems(obj.elems, heap, 1)}]`;
      if (obj.entries?.length) return `{…${obj.entries.length}}`;
      return `#${v.value}`;
    }
    case "Ref":
    case "Ptr":
      return formatAddress(v.value, heap, 0);
    default:
      return "…";
  }
}

export function chipTitle(
  value: ValueJson | { kind: string; value?: unknown },
  heap?: HeapSnapshot,
): string {
  return `${formatChip(value, heap)} ${JSON.stringify(value)}`;
}
