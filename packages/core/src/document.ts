/**
 * Document model. The runtime representation is a *flat* node table keyed by id
 * (O(1) lookup, cheap patches); tree structure is expressed via `parentId`+`order`.
 */
export interface Node {
  id: string;
  type: string;
  parentId: string | null;
  order: number;
  enabled: boolean;
  opacity: number;
  props: Record<string, unknown>;
}

export interface DocumentState {
  schemaVersion: number;
  appId: string;
  /** Monotonic version used for optimistic-concurrency (CAS). */
  docVersion: number;
  selection: string[];
  nodes: Record<string, Node>;
}

export const DOCUMENT_SCHEMA_VERSION = 1;

export function createDocument(appId: string): DocumentState {
  return {
    schemaVersion: DOCUMENT_SCHEMA_VERSION,
    appId,
    docVersion: 0,
    selection: [],
    nodes: {},
  };
}

/** Nodes in sibling order (stable, deterministic). */
export function listNodes(state: DocumentState): Node[] {
  return Object.values(state.nodes).sort((a, b) => a.order - b.order);
}
