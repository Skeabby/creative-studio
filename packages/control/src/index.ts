import { CommandBus, NucleusError, type DocumentState, type ErrorKind } from "@nucleus/core";

/**
 * Transport-agnostic control surface. CLI, JSON-RPC and MCP frontends are all
 * thin adapters over `handle()`. No business logic lives in a frontend.
 */
export interface ControlRequest {
  method: string;
  params?: unknown;
}

export interface ControlFailure {
  kind: ErrorKind;
  message: string;
}

export interface ControlResponse {
  ok: boolean;
  result?: unknown;
  error?: ControlFailure;
}

export class ControlServer {
  readonly #bus: CommandBus;
  readonly #state: DocumentState;

  constructor(bus: CommandBus, state: DocumentState) {
    this.#bus = bus;
    this.#state = state;
  }

  get bus(): CommandBus {
    return this.#bus;
  }

  get state(): DocumentState {
    return this.#state;
  }

  handle(request: ControlRequest): ControlResponse {
    try {
      switch (request.method) {
        case "command.list":
          return { ok: true, result: this.#bus.list() };

        case "command.describe": {
          const name = (request.params as { name?: string } | undefined)?.name;
          if (!name) throw new NucleusError("validation", "command.describe requires params.name");
          const d = this.#bus.describe(name);
          return {
            ok: true,
            result: {
              name: d.name,
              title: d.title,
              description: d.description,
              sideEffects: d.sideEffects,
              reversible: d.reversible ?? true,
            },
          };
        }

        case "command.invoke": {
          const { name, params } = (request.params ?? {}) as { name?: string; params?: unknown };
          if (!name) throw new NucleusError("validation", "command.invoke requires params.name");
          const result = this.#bus.invoke(name, this.#state, params);
          return { ok: true, result };
        }

        case "doc.get":
          return { ok: true, result: this.#state };

        case "history.status":
          return { ok: true, result: { canUndo: this.#bus.canUndo(), canRedo: this.#bus.canRedo() } };

        case "history.undo":
          return { ok: true, result: { undone: this.#bus.undo(this.#state), docVersion: this.#state.docVersion } };

        case "history.redo":
          return { ok: true, result: { redone: this.#bus.redo(this.#state), docVersion: this.#state.docVersion } };

        default:
          throw new NucleusError("not_found", `unknown method: ${request.method}`);
      }
    } catch (error) {
      const nucleusError =
        error instanceof NucleusError
          ? error
          : new NucleusError("internal", error instanceof Error ? error.message : String(error));
      return { ok: false, error: { kind: nucleusError.kind, message: nucleusError.message } };
    }
  }
}
