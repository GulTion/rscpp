/**
 * rscpp spans are UTF-8 **byte** offsets ([docs/events.md](spans)).
 * CodeMirror positions are UTF-16 code-unit indices into the JS string.
 */

function utf8Len(cp: number): number {
  if (cp <= 0x7f) return 1;
  if (cp <= 0x7ff) return 2;
  if (cp <= 0xffff) return 3;
  return 4;
}

/** map[i] = UTF-8 byte offset of the code unit at index i; map[length] = total bytes. */
export function buildByteIndexMap(s: string): Uint32Array {
  const map = new Uint32Array(s.length + 1);
  let bytes = 0;
  let i = 0;
  while (i < s.length) {
    const cp = s.codePointAt(i)!;
    const w = cp > 0xffff ? 2 : 1;
    map[i] = bytes;
    if (w === 2) map[i + 1] = bytes; // low surrogate shares start byte
    bytes += utf8Len(cp);
    i += w;
  }
  map[s.length] = bytes;
  return map;
}

/** Largest JS index whose starting byte offset is ≤ byteOffset. */
export function byteToJs(map: Uint32Array, byteOffset: number): number {
  const target = Math.max(0, byteOffset);
  let lo = 0;
  let hi = map.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (map[mid] <= target) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

export function jsToByte(map: Uint32Array, jsIndex: number): number {
  const i = Math.max(0, Math.min(jsIndex, map.length - 1));
  return map[i];
}

export function spanBytesToJs(
  map: Uint32Array,
  start: number,
  end: number,
): { from: number; to: number } {
  let from = byteToJs(map, start);
  let to = byteToJs(map, end);
  if (to < from) to = from;
  return { from, to };
}
