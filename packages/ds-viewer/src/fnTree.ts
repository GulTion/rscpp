import type { EventJson, ValueJson } from "@rscpp/timeline";

export const FN_TREE_PANE_ID = -1;

export type FnArg = {
  name: string;
  value: ValueJson;
};

export type FnTreeNode = {
  call_id: number;
  parent_id: number | null;
  name: string;
  args: FnArg[];
  enterIndex: number;
  exitIndex: number | null;
  /** Still on the stack at the playhead. */
  active: boolean;
  /** Current event is this call's FnExit (red flash before done/grey). */
  exiting: boolean;
  children: FnTreeNode[];
};

function shortName(name: string): string {
  const i = name.lastIndexOf("::");
  return i >= 0 ? name.slice(i + 2) : name;
}

/** Param names: VarCreate after FnEnter, in order, until ScopeEnter / Fn*. */
export function argNamesAfterEnter(events: EventJson[], enterIndex: number): string[] {
  const names: string[] = [];
  for (let i = enterIndex + 1; i < events.length; i++) {
    const ev = events[i];
    if (
      ev.kind === "ScopeEnter" ||
      ev.kind === "FnEnter" ||
      ev.kind === "FnExit"
    ) {
      break;
    }
    if (ev.kind === "VarCreate" && typeof ev.name === "string" && ev.name) {
      names.push(ev.name);
    }
  }
  return names;
}

export function formatArgValue(v: ValueJson): string {
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
    case "Ref":
    case "Ptr": {
      const addr = v.value as { kind?: string; value?: unknown } | null;
      if (addr && addr.kind === "Heap" && typeof addr.value === "number") {
        return `#${addr.value}`;
      }
      return v.kind === "Ref" ? "ref" : "ptr";
    }
    default:
      return "…";
  }
}

export function objectIdOfArg(v: ValueJson): number | null {
  if (v.kind === "Object") return v.value;
  if (v.kind === "Ref" || v.kind === "Ptr") {
    const addr = v.value as { kind?: string; value?: unknown } | null;
    if (addr && addr.kind === "Heap" && typeof addr.value === "number") {
      return addr.value;
    }
  }
  return null;
}

function isScalarArg(v: ValueJson): boolean {
  return (
    v.kind === "Int" ||
    v.kind === "Float" ||
    v.kind === "Bool" ||
    v.kind === "Char" ||
    v.kind === "Str"
  );
}

/**
 * Args shown on the tree: scalars that changed vs parent.
 * Heap Object/Ref/Ptr args (e.g. vis, adj) are never shown — they stay shared.
 */
export function changingArgs(
  node: FnTreeNode,
  parent: FnTreeNode | null,
): FnArg[] {
  return node.args.filter((a) => {
    if (!isScalarArg(a.value)) return false;
    if (!parent) return true;
    return !argUnchangedFromParent(parent, a);
  });
}

export function formatFnLabel(
  node: FnTreeNode,
  parent: FnTreeNode | null = null,
): string {
  const args = changingArgs(node, parent)
    .map((a) => `${a.name}=${formatArgValue(a.value)}`)
    .join(", ");
  const base = shortName(node.name);
  return args ? `${base}(${args})` : `${base}()`;
}

/**
 * Call tree visible at playhead `timelineIndex` (events applied in `[0, t)`).
 */
export function buildFnTree(
  events: EventJson[],
  timelineIndex: number,
): FnTreeNode[] {
  const t = Math.max(0, Math.min(timelineIndex, events.length));

  type Rec = {
    call_id: number;
    parent_id: number | null;
    name: string;
    args: FnArg[];
    enterIndex: number;
    exitIndex: number | null;
  };
  const recs = new Map<number, Rec>();

  for (let i = 0; i < events.length; i++) {
    const ev = events[i];
    if (ev.kind === "FnEnter") {
      const call_id = ev.call_id as number;
      const parent_id =
        ev.parent_id === undefined || ev.parent_id === null
          ? null
          : (ev.parent_id as number);
      const rawArgs = (ev.args as ValueJson[] | undefined) ?? [];
      const names = argNamesAfterEnter(events, i);
      recs.set(call_id, {
        call_id,
        parent_id,
        name: String(ev.name ?? "?"),
        args: rawArgs.map((value, j) => ({
          name: names[j] ?? `arg${j}`,
          value,
        })),
        enterIndex: i,
        exitIndex: null,
      });
    } else if (ev.kind === "FnExit") {
      const n = recs.get(ev.call_id as number);
      if (n && n.exitIndex === null) n.exitIndex = i;
    }
  }

  const liveById = new Map<number, FnTreeNode>();
  for (const r of recs.values()) {
    if (r.enterIndex >= t) continue;
    const exited = r.exitIndex !== null && r.exitIndex < t;
    const exiting = r.exitIndex !== null && r.exitIndex === t - 1;
    liveById.set(r.call_id, {
      call_id: r.call_id,
      parent_id: r.parent_id,
      name: r.name,
      args: r.args,
      enterIndex: r.enterIndex,
      exitIndex: r.exitIndex,
      active: !exited,
      exiting,
      children: [],
    });
  }

  const roots: FnTreeNode[] = [];
  for (const live of liveById.values()) {
    if (live.parent_id !== null && liveById.has(live.parent_id)) {
      liveById.get(live.parent_id)!.children.push(live);
    } else {
      roots.push(live);
    }
  }

  const sortRec = (n: FnTreeNode) => {
    n.children.sort((a, b) => a.enterIndex - b.enterIndex);
    n.children.forEach(sortRec);
  };
  roots.sort((a, b) => a.enterIndex - b.enterIndex);
  roots.forEach(sortRec);
  return roots;
}

/** Whether this arg looks unchanged vs the same-named arg on the parent call. */
export function argUnchangedFromParent(
  parent: FnTreeNode | null,
  arg: FnArg,
): boolean {
  if (!parent) return false;
  const p = parent.args.find((a) => a.name === arg.name);
  if (!p) return false;
  const id = objectIdOfArg(arg.value);
  const pid = objectIdOfArg(p.value);
  if (id !== null && pid !== null) return id === pid;
  return JSON.stringify(arg.value) === JSON.stringify(p.value);
}
