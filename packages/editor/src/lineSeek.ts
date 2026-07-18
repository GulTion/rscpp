import type { EventJson } from "@rscpp/timeline";
import { buildByteIndexMap, jsToByte } from "./spans.js";

/** 0-based line → UTF-8 byte [start, end) in source. */
export function lineByteRange(
  source: string,
  lineIndex0: number,
): { start: number; end: number } | null {
  const map = buildByteIndexMap(source);
  let line = 0;
  let startJs = 0;
  for (let i = 0; i <= source.length; i++) {
    if (i === source.length || source[i] === "\n") {
      if (line === lineIndex0) {
        const endJs = i === source.length ? i : i + 1;
        return {
          start: jsToByte(map, startJs),
          end: jsToByte(map, endJs),
        };
      }
      line++;
      startJs = i + 1;
    }
  }
  return null;
}

/** First event index whose span overlaps the given 0-based line, or null. */
export function eventIndexForLine(
  events: EventJson[],
  source: string,
  lineIndex0: number,
): number | null {
  const range = lineByteRange(source, lineIndex0);
  if (!range) return null;
  for (let i = 0; i < events.length; i++) {
    const sp = events[i].span;
    if (!sp || sp.end <= sp.start) continue;
    if (sp.start < range.end && sp.end > range.start) return i;
  }
  return null;
}
