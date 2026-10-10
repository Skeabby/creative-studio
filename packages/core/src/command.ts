import type { DocumentState, Node } from "./document.js";
import { StudioError } from "./errors.js";

export type SideEffects = "read" | "write" | "destructive";

export interface Actor {
  kind: "human" | "agent" | "system";
  id: string;
  session?: string;
}

/** A reversible edit. Every mutation goes through patches. */
export interface Patch {
  op: "add" | "remove" | "update";
  nodeId: string;
  before?: Node;
  after?: Node;
}

export interface CommandContext {
  bus: CommandBus;
  actor: Actor;
  state: DocumentState;
}

export interface CommandOutcome<R> {
  patches?: Patch[];
  result: R;
}

/**
 * A self-describing command. Deliberately pure: it computes patches and a
 * result, but never mutates the document itself.
 */
export interface CommandDescriptor<P = unknown, R = unknown> {
  name: string;
  title: string;
  description: string;
  sideEffects: SideEffects;
  reversible?: boolean;
  run: (params: P, ctx: CommandContext) => CommandOutcome<R>;
}

export interface InvokeOptions {
  actor?: Actor;
  dryRun?: boolean;
}

export interface InvokeResult<R = unknown> {
  result: R;
  applied: number;
  docVersion: number;
}

export class CommandBus {
  readonly #handlers = new Map<string, CommandDescriptor<any, any>>();
  readonly #history: Patch[][] = [];
  readonly #redo: Patch[][] = [];
  readonly #defaultActor: Actor;

  constructor(defaultActor: Actor = { kind: "system", id: "core" }) {
    this.#defaultActor = defaultActor;
  }

  register<P, R>(descriptor: CommandDescriptor<P, R>): void {
    if (this.#handlers.has(descriptor.name)) {
      throw new StudioError("internal", `duplicate command: ${descriptor.name}`);
    }
    this.#handlers.set(descriptor.name, descriptor as CommandDescriptor<any, any>);
  }

  has(name: string): boolean {
    return this.#handlers.has(name);
  }

  list(): string[] {
    return [...this.#handlers.keys()].sort();
  }

  describe(name: string): CommandDescriptor<any, any> {
    const descriptor = this.#handlers.get(name);
    if (!descriptor) throw new StudioError("not_found", `unknown command: ${name}`);
    return descriptor;
  }

  invoke<R = unknown>(
    name: string,
    state: DocumentState,
    params: unknown = {},
    options: InvokeOptions = {},
  ): InvokeResult<R> {
    const descriptor = this.describe(name);
    const actor = options.actor ?? this.#defaultActor;
    const outcome = descriptor.run(params, { bus: this, actor, state });
    const patches = outcome.patches ?? [];

    if (!options.dryRun && patches.length > 0) {
      const inverse = applyPatches(state, patches);
      if (descriptor.reversible !== false) this.#history.push(inverse);
      // A fresh edit invalidates any redo branch.
      this.#redo.length = 0;
      state.docVersion += 1;
    }

    return { result: outcome.result as R, applied: patches.length, docVersion: state.docVersion };
  }

  canUndo(): boolean {
    return this.#history.length > 0;
  }

  canRedo(): boolean {
    return this.#redo.length > 0;
  }

  undo(state: DocumentState): boolean {
    const inverse = this.#history.pop();
    if (!inverse) return false;
    const redo = applyPatches(state, inverse);
    this.#redo.push(redo);
    state.docVersion += 1;
    return true;
  }

  redo(state: DocumentState): boolean {
    const redo = this.#redo.pop();
    if (!redo) return false;
    const inverse = applyPatches(state, redo);
    this.#history.push(inverse);
    state.docVersion += 1;
    return true;
  }
}

/**
 * Applies patches in place and returns the inverse patches (for undo).
 * The inverse list is reversed so it can be replayed directly to restore state.
 */
export function applyPatches(state: DocumentState, patches: Patch[]): Patch[] {
  const inverse: Patch[] = [];

  for (const patch of patches) {
    const before = state.nodes[patch.nodeId];

    if (patch.op === "remove") {
      if (before) {
        delete state.nodes[patch.nodeId];
        inverse.push({ op: "add", nodeId: patch.nodeId, after: before });
      }
      continue;
    }

    const after = patch.after ?? before;
    if (!after) continue;
    state.nodes[patch.nodeId] = after;
    inverse.push(
      before
        ? { op: "update", nodeId: patch.nodeId, after: before }
        : { op: "remove", nodeId: patch.nodeId },
    );
  }

  return inverse.reverse();
}
