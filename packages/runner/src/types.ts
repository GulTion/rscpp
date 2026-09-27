import type { EventJson } from "@rscpp/timeline";

export type RunError = {
  message: string;
  span?: { start: number; end: number };
  /** Multi-line location snippet from WASM when source was available. */
  formatted?: string;
};

export type RunResult = {
  ok: boolean;
  value?: unknown;
  events: EventJson[];
  error?: RunError;
};
