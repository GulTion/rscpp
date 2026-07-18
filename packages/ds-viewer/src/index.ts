export { mountDsViewer, listAllocIds, listLiveIds } from "./mount.js";
export type { DsViewerProps, MountHandle } from "./mount.js";
export {
  proposeRepresentations,
  representationLabel,
  isGraphEncoding,
  GRAPH_ENCODINGS,
  normalizeRepresentation,
} from "./represent.js";
export type { Representation, GraphEncoding } from "./represent.js";
export { diffElems, formatVal } from "./diff.js";
export { bindingsFromSnapshot, objectIdOf } from "./bindings.js";
export type { VarBinding } from "./bindings.js";
export { normalizeEdges, DEFAULT_GRAPH_OPTS } from "./graphOpts.js";
export type { GraphViewOpts, GraphDirection, DrawEdge } from "./graphOpts.js";
export { walkHighlight } from "./walk.js";
export type { WalkHighlight } from "./walk.js";
export { buildFnTree, FN_TREE_PANE_ID, formatFnLabel } from "./fnTree.js";
export type { FnTreeNode, FnArg } from "./fnTree.js";
export { setMathContent, toTex, MATH_FONT } from "./math.js";
