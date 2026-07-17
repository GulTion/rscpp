import type {
  EventJson,
  FrameState,
  HeapSnapshot,
  ObjectState,
  ValueJson,
} from "./types.js";

function cloneValue(v: ValueJson): ValueJson {
  return structuredClone(v);
}

function cloneObject(o: ObjectState): ObjectState {
  return {
    type_name: o.type_name,
    elems: o.elems?.map(cloneValue),
    entries: o.entries?.map((e) => ({
      key: structuredClone(e.key),
      value: e.value !== undefined ? cloneValue(e.value) : undefined,
    })),
  };
}

function emptySnapshot(): HeapSnapshot {
  return {
    objects: new Map(),
    frames: [],
    openLoops: [],
  };
}

function topFrame(frames: FrameState[]): FrameState | undefined {
  return frames[frames.length - 1];
}

function setLocal(frames: FrameState[], name: string, value: ValueJson): void {
  const f = topFrame(frames);
  if (f) f.locals.set(name, cloneValue(value));
}

function objId(container: unknown): number | null {
  if (
    container &&
    typeof container === "object" &&
    (container as { kind?: string }).kind === "Object" &&
    typeof (container as { value?: unknown }).value === "number"
  ) {
    return (container as { value: number }).value;
  }
  return null;
}

function applyContainerMod(obj: ObjectState, ev: EventJson): void {
  const op = String(ev.op ?? "");
  const bulk = ev.elems as ValueJson[] | undefined;
  if (Array.isArray(bulk) && bulk.length > 0) {
    obj.elems = bulk.map(cloneValue);
    return;
  }
  if (!obj.elems) obj.elems = [];

  switch (op) {
    case "push_back":
    case "emplace_back":
    case "stack::push":
    case "stack::emplace":
    case "queue::push":
    case "queue::emplace":
      if (ev.value !== undefined) obj.elems.push(cloneValue(ev.value as ValueJson));
      break;
    case "pop_back":
    case "stack::pop":
      obj.elems.pop();
      break;
    case "queue::pop":
      obj.elems.shift();
      break;
    case "map_default_insert":
    case "map_assign":
    case "set::insert":
    case "set::emplace": {
      if (!obj.entries) obj.entries = [];
      const key = ev.key;
      const value = ev.value as ValueJson | undefined;
      const i = obj.entries.findIndex(
        (e) => JSON.stringify(e.key) === JSON.stringify(key),
      );
      if (i >= 0) {
        if (value !== undefined) obj.entries[i].value = cloneValue(value);
      } else {
        obj.entries.push({
          key: structuredClone(key),
          value: value !== undefined ? cloneValue(value) : undefined,
        });
      }
      break;
    }
    default: {
      const index = ev.index;
      if (typeof index === "number" && ev.value !== undefined && obj.elems) {
        obj.elems[index] = cloneValue(ev.value as ValueJson);
      }
      break;
    }
  }
}

function applyWrite(snap: HeapSnapshot, ev: EventJson): void {
  const slot = ev.slot as { kind?: string; name?: string; obj?: number; index?: number; key?: unknown; field?: string } | undefined;
  const value = ev.value as ValueJson | undefined;
  if (!slot || value === undefined) return;

  switch (slot.kind) {
    case "Local":
    case "Global":
      if (slot.name) setLocal(snap.frames, slot.name, value);
      break;
    case "Index":
      if (typeof slot.obj === "number" && typeof slot.index === "number") {
        const o = snap.objects.get(slot.obj);
        if (o) {
          if (!o.elems) o.elems = [];
          o.elems[slot.index] = cloneValue(value);
        }
      }
      break;
    case "MapEntry":
      if (typeof slot.obj === "number") {
        const o = snap.objects.get(slot.obj);
        if (o) {
          if (!o.entries) o.entries = [];
          const i = o.entries.findIndex(
            (e) => JSON.stringify(e.key) === JSON.stringify(slot.key),
          );
          if (i >= 0) o.entries[i].value = cloneValue(value);
          else o.entries.push({ key: structuredClone(slot.key), value: cloneValue(value) });
        }
      }
      break;
    case "Object":
    case "Field":
      // Field writes are rare in v1; treat Object slot as no-op on elems
      break;
  }
}

function applyEvent(snap: HeapSnapshot, ev: EventJson): void {
  switch (ev.kind) {
    case "Alloc": {
      const id = ev.id as number;
      const type_name = String(ev.type_name ?? "object");
      const state: ObjectState = { type_name };
      if (Array.isArray(ev.elems)) state.elems = (ev.elems as ValueJson[]).map(cloneValue);
      if (Array.isArray(ev.entries)) {
        state.entries = (ev.entries as { key: unknown; value?: ValueJson }[]).map((e) => ({
          key: structuredClone(e.key),
          value: e.value !== undefined ? cloneValue(e.value) : undefined,
        }));
      }
      if (!state.elems && !state.entries) state.elems = [];
      snap.objects.set(id, state);
      break;
    }
    case "Dealloc": {
      snap.objects.delete(ev.id as number);
      break;
    }
    case "Write":
      applyWrite(snap, ev);
      break;
    case "VarCreate": {
      const name = ev.name as string;
      const value = ev.value as ValueJson;
      const f = topFrame(snap.frames);
      if (f && name && value !== undefined && !f.locals.has(name)) {
        f.locals.set(name, cloneValue(value));
      }
      break;
    }
    case "VarAssign":
      // Prefer Write — skip to avoid double-update when both fire
      break;
    case "ContainerMod": {
      const id = objId(ev.container);
      if (id !== null) {
        const o = snap.objects.get(id);
        if (o) applyContainerMod(o, ev);
      }
      break;
    }
    case "FnEnter": {
      snap.frames.push({
        call_id: ev.call_id as number,
        name: String(ev.name ?? ""),
        parent_id: (ev.parent_id as number | null | undefined) ?? null,
        locals: new Map(),
      });
      const args = ev.args as ValueJson[] | undefined;
      // args are positional; names unknown — skip
      void args;
      break;
    }
    case "FnExit": {
      const call_id = ev.call_id as number;
      while (snap.frames.length && snap.frames[snap.frames.length - 1].call_id !== call_id) {
        snap.frames.pop();
      }
      snap.frames.pop();
      break;
    }
    case "LoopIter": {
      const id = ev.loop_id as number;
      if (!snap.openLoops.includes(id)) snap.openLoops.push(id);
      break;
    }
    case "LoopEnd": {
      const id = ev.loop_id as number;
      const i = snap.openLoops.lastIndexOf(id);
      if (i >= 0) snap.openLoops.splice(i, 1);
      break;
    }
    case "Break":
    case "Continue":
      // Break is followed by LoopEnd; Continue keeps instance open
      break;
    default:
      break;
  }
}

/** State after the first `t` events (`t === 0` → empty). Full replay from 0. */
export function reconstruct(events: EventJson[], t: number): HeapSnapshot {
  const n = Math.max(0, Math.min(t, events.length));
  const snap = emptySnapshot();
  for (let i = 0; i < n; i++) applyEvent(snap, events[i]);
  return snap;
}

export function cloneSnapshot(s: HeapSnapshot): HeapSnapshot {
  return {
    objects: new Map([...s.objects.entries()].map(([k, v]) => [k, cloneObject(v)])),
    frames: s.frames.map((f) => ({
      call_id: f.call_id,
      name: f.name,
      parent_id: f.parent_id,
      locals: new Map([...f.locals.entries()].map(([k, v]) => [k, cloneValue(v)])),
    })),
    openLoops: [...s.openLoops],
  };
}
