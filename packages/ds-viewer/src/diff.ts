import type { ObjectState, ValueJson } from "@rscpp/timeline";

export type CellDiff = {
  index: number;
  kind: "add" | "remove" | "change" | "same";
  value?: ValueJson;
};

export function diffElems(
  prev: ObjectState | undefined,
  next: ObjectState | undefined,
): CellDiff[] {
  const a = prev?.elems ?? [];
  const b = next?.elems ?? [];
  const n = Math.max(a.length, b.length);
  const out: CellDiff[] = [];
  for (let i = 0; i < n; i++) {
    if (i >= a.length) out.push({ index: i, kind: "add", value: b[i] });
    else if (i >= b.length) out.push({ index: i, kind: "remove", value: a[i] });
    else if (JSON.stringify(a[i]) !== JSON.stringify(b[i]))
      out.push({ index: i, kind: "change", value: b[i] });
    else out.push({ index: i, kind: "same", value: b[i] });
  }
  return out;
}

export function formatVal(v: ValueJson | undefined): string {
  if (!v) return "";
  switch (v.kind) {
    case "Int":
    case "Float":
    case "Bool":
    case "Char":
    case "Str":
      return String(v.value);
    case "Object":
      return `#${v.value}`;
    case "Nullptr":
      return "nullptr";
    case "Void":
      return "void";
    default:
      return v.kind;
  }
}
