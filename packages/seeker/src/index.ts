export { mountSeeker } from "./mount.js";
export type { SeekerProps, MountHandle } from "./mount.js";
export {
  buildLoopSegments,
  buildCallSegments,
  colorIndexForName,
  currentSegment,
} from "./segments.js";
export type { LoopSegment, CallSegment, ActiveSegment } from "./segments.js";
export {
  countVisibleInRange,
  playheadToVisualInRange,
  visualToPlayheadInRange,
} from "./detailAxis.js";
export {
  LAYER_H,
  allSpans,
  callStackDepth,
  loopStackDepth,
  maxStackDepth,
  slabBottom,
  stackHeight,
  containmentDepth,
} from "./stackLayout.js";
