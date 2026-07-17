export type Span = { start: number; end: number };

export type ValueJson =
  | { kind: "Void" }
  | { kind: "Bool"; value: boolean }
  | { kind: "Int"; value: number }
  | { kind: "Float"; value: number }
  | { kind: "Char"; value: string }
  | { kind: "Nullptr" }
  | { kind: "Object"; value: number }
  | { kind: "Str"; value: string }
  | { kind: "Ptr"; value: unknown }
  | { kind: "Ref"; value: unknown };

export type EventJson = { kind: string; span?: Span; [k: string]: unknown };

export type ObjectState = {
  type_name: string;
  elems?: ValueJson[];
  entries?: { key: unknown; value?: ValueJson }[];
};

export type FrameState = {
  call_id: number;
  name: string;
  parent_id: number | null;
  locals: Map<string, ValueJson>;
};

export type HeapSnapshot = {
  objects: Map<number, ObjectState>;
  frames: FrameState[];
  openLoops: number[];
};

export type HighlightRange = { start: number; end: number; kind: string };

export type TimelineEvent =
  | { type: "tick"; index: number; snapshot: HeapSnapshot }
  | { type: "highlight"; ranges: HighlightRange[] }
  | { type: "seek"; index: number };

export type Timeline = {
  readonly length: number;
  readonly index: number;
  readonly source: string;
  readonly events: EventJson[];
  seek(t: number): void;
  step(delta: number): void;
  play(opts?: { speed?: number }): void;
  pause(): void;
  /** Whether play() interval is active. */
  readonly playing: boolean;
  snapshot(): HeapSnapshot;
  highlight(): HighlightRange[];
  setHoverHighlight(ranges: HighlightRange[] | null): void;
  subscribe(listener: (ev: TimelineEvent) => void): () => void;
};
