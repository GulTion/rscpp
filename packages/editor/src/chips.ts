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
      return `⦃${v.value}⦄`;
    case "Void":
      return "⦃void⦄";
    case "Nullptr":
      return "⦃nullptr⦄";
    case "Object": {
      const obj = heap?.objects.get(v.value);
      if (obj?.elems) return `⦃[${formatElems(obj.elems, heap, 1)}]⦄`;
      if (obj?.entries?.length) return `⦃{…${obj.entries.length}}⦄`;
      return `⦃#${v.value}⦄`;
    }
    case "Ref":
    case "Ptr":
      return `⦃${formatAddress(v.value, heap, 0)}⦄`;
    default:
      return "⦃…⦄";
  }
}

export function chipTitle(
  value: ValueJson | { kind: string; value?: unknown },
  heap?: HeapSnapshot,
): string {
  return `${formatChip(value, heap)} ${JSON.stringify(value)}`;
}
