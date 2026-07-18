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
