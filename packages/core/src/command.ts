import type { DocumentState, Node } from "./document.js";
import { NucleusError } from "./errors.js";

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
  /**
   * Merge this command into the previous history entry when both share the key
   * and land within the coalescing window (F2). Used so a burst of autosaves
   * collapses into a single undo instead of one step per keystroke.
   */
  coalesceKey?: string;
  /** Window (ms) in which a shared `coalesceKey` merges. Default 800. */
  coalesceWindowMs?: number;
}

export interface InvokeResult<R = unknown> {
  result: R;
  applied: number;
  docVersion: number;
}

/** One undo/redo entry. `key`/`at` drive coalescing of bursts (F2). */
interface HistoryEntry {
  patches: Patch[];
  key?: string;
  at: number;
}

/**
 * Merge a later inverse onto an earlier one for coalescing. The earliest patch
 * per node wins, so undoing the merged entry restores the state from before the
 * *first* command in the burst.
 */
function mergeInverse(earlier: Patch[], later: Patch[]): Patch[] {
  const byNode = new Map<string, Patch>();
  for (const patch of later) byNode.set(patch.nodeId, patch);
  for (const patch of earlier) byNode.set(patch.nodeId, patch);
  return [...byNode.values()];
}

export class CommandBus {
  readonly #handlers = new Map<string, CommandDescriptor<any, any>>();
  readonly #history: HistoryEntry[] = [];
  readonly #redo: HistoryEntry[] = [];
  readonly #defaultActor: Actor;

  constructor(defaultActor: Actor = { kind: "system", id: "core" }) {
    this.#defaultActor = defaultActor;
  }

  register<P, R>(descriptor: CommandDescriptor<P, R>): void {
    if (this.#handlers.has(descriptor.name)) {
      throw new NucleusError("internal", `duplicate command: ${descriptor.name}`);
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
    if (!descriptor) throw new NucleusError("not_found", `unknown command: ${name}`);
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
      if (descriptor.reversible !== false) this.#pushHistory(inverse, options);
      // A fresh edit invalidates any redo branch.
      this.#redo.length = 0;
      state.docVersion += 1;
    }

    return { result: outcome.result as R, applied: patches.length, docVersion: state.docVersion };
  }

  /**
   * Record an undo entry, coalescing into the previous one when the caller
   * supplied a `coalesceKey` that matches and we are still inside the window.
   */
  #pushHistory(patches: Patch[], options: InvokeOptions): void {
    const now = Date.now();
    const window = options.coalesceWindowMs ?? 800;
    const last = this.#history[this.#history.length - 1];
    const key = options.coalesceKey;
    if (
      key !== undefined &&
      last !== undefined &&
      last.key === key &&
      now - last.at <= window
    ) {
      last.patches = mergeInverse(last.patches, patches);
      last.at = now;
      return;
    }
    this.#history.push({ patches, key, at: now });
  }

  canUndo(): boolean {
    return this.#history.length > 0;
  }

  canRedo(): boolean {
    return this.#redo.length > 0;
  }

  undo(state: DocumentState): boolean {
    const entry = this.#history.pop();
    if (!entry) return false;
    const redo = applyPatches(state, entry.patches);
    this.#redo.push({ patches: redo, at: Date.now() });
    state.docVersion += 1;
    return true;
  }

  redo(state: DocumentState): boolean {
    const entry = this.#redo.pop();
    if (!entry) return false;
    const inverse = applyPatches(state, entry.patches);
    this.#history.push({ patches: inverse, at: Date.now() });
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
