import type { Timeline, HeapSnapshot, HighlightRange } from "@rscpp/timeline";
import type { RunResult } from "@rscpp/runner";

export type RscppDebug = {
  timeline: Timeline;
  lastRun: RunResult | null;
  seek(t: number): void;
  snapshot(): HeapSnapshot;
  highlight(): HighlightRange[];
};

export function installDebug(api: RscppDebug): void {
  (window as unknown as { __rscppDebug: RscppDebug }).__rscppDebug = api;
}
