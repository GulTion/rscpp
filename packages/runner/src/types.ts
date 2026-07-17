import type { EventJson } from "@rscpp/timeline";

export type RunError = {
  message: string;
  span?: { start: number; end: number };
};

export type RunResult = {
  ok: boolean;
  value?: unknown;
  events: EventJson[];
  error?: RunError;
};
