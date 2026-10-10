/**
 * Stable, machine-readable error taxonomy shared across the command bus,
 * the JSON-RPC control layer and the MCP adapter.
 */
export type ErrorKind =
  | "validation"
  | "not_found"
  | "conflict"
  | "unauthorized"
  | "unsupported"
  | "internal"
  | "cancelled";

/** JSON-RPC codes. -32000..-32099 is reserved for the studio server. */
export const JSON_RPC_CODES: Record<ErrorKind, number> = {
  validation: -32602,
  not_found: -32001,
  conflict: -32002,
  unauthorized: -32003,
  unsupported: -32004,
  cancelled: -32005,
  internal: -32603,
};

export class StudioError extends Error {
  readonly kind: ErrorKind;
  readonly code: number;
  readonly details: unknown;

  constructor(kind: ErrorKind, message: string, details?: unknown) {
    super(message);
    this.name = "StudioError";
    this.kind = kind;
    this.code = JSON_RPC_CODES[kind];
    this.details = details;
  }
}
