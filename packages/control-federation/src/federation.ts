/**
 * @nucleus/control-federation — one agent, many apps.
 *
 * Each Nucleus app already owns a `CommandBus` + `ControlServer` (see
 * `@nucleus/pdf`, `@nucleus/notes`). The federation joins those per-app
 * surfaces into a single one:
 *
 *   - unified command list  (`pdf.importPages`, `note.create`, …)
 *   - unified invocation    (routes to the owning app)
 *   - unified undo/redo     (one journal, applied across apps in reverse order)
 *
 * That last point is the POC's whole bet: an agent edits app A, then app B,
 * and a single undo unwinds both — in the right order, per app.
 */
import { NucleusError, type CommandBus, type DocumentState } from "@nucleus/core";
import type { ControlServer } from "@nucleus/control";

/** An operation that is not a bus command (file I/O: load, export, …). */
export interface MemberTool {
  /** Globally unique, namespaced name — e.g. `pdf.load`. */
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  run: (args: Record<string, unknown>) => unknown | Promise<unknown>;
}

/** One app joined into the federation. */
export interface Member {
  /** Stable app id from the manifest, e.g. `nucleus.pdf`. */
  id: string;
  /** Short namespace used to qualify commands, e.g. `pdf`. */
  prefix: string;
  bus: CommandBus;
  state: DocumentState;
  control: ControlServer;
  /** Read-only view of the app state, for the agent. */
  summary?: () => unknown;
  /** Non-command operations owned by the app. */
  tools?: MemberTool[];
}

export interface CommandInfo {
  /** Canonical invocation name — `note.create`, or `pdf/note.create` when ambiguous. */
  name: string;
  localName: string;
  member: string;
  prefix: string;
  title: string;
  description: string;
  sideEffects: string;
  reversible: boolean;
}

export interface JournalEntry {
  /** Monotonic id — stable across undo/redo. */
  id: number;
  member: string;
  prefix: string;
  /** Name the caller used. */
  command: string;
  localName: string;
  title: string;
}

export interface InvokeOutcome<R = unknown> {
  command: string;
  localName: string;
  member: string;
  prefix: string;
  result: R;
  applied: number;
  docVersion: number;
  /** `null` for read-only commands (no patch applied, nothing to undo). */
  journalId: number | null;
}

export interface UndoOutcome {
  changed: boolean;
  entry?: JournalEntry;
  /** Present when `changed` is false. */
  reason?: string;
}

export interface HistoryStatus {
  canUndo: boolean;
  canRedo: boolean;
  depth: number;
  redoDepth: number;
  undo: JournalEntry[];
  redo: JournalEntry[];
}

export interface MemberState {
  id: string;
  prefix: string;
  commands: number;
  tools: string[];
  docVersion: number;
  summary: unknown;
}

const PREFIX_RE = /^[a-z][a-z0-9_-]*$/;

export class Federation {
  readonly #members = new Map<string, Member>();
  #journal: JournalEntry[] = [];
  #redo: JournalEntry[] = [];
  #ticks = 0;

  /** Join an app. Prefixes and ids must be unique. */
  register(member: Member): this {
    if (!PREFIX_RE.test(member.prefix)) {
      throw new NucleusError("validation", `invalid member prefix: ${member.prefix}`);
    }
    if (this.#members.has(member.prefix)) {
      throw new NucleusError("conflict", `member prefix already registered: ${member.prefix}`);
    }
    for (const existing of this.#members.values()) {
      if (existing.id === member.id) {
        throw new NucleusError("conflict", `member id already registered: ${member.id}`);
      }
      for (const tool of member.tools ?? []) {
        if ((existing.tools ?? []).some((t) => t.name === tool.name)) {
          throw new NucleusError("conflict", `tool name already registered: ${tool.name}`);
        }
      }
    }
    this.#members.set(member.prefix, member);
    return this;
  }

  /** Joined apps, in registration order. */
  members(): Member[] {
    return [...this.#members.values()];
  }

  prefixes(): string[] {
    return [...this.#members.keys()];
  }

  /** Every command of every app, with its canonical invocation name. */
  commands(): CommandInfo[] {
    const out: CommandInfo[] = [];
    for (const member of this.#members.values()) {
      for (const localName of member.bus.list()) {
        const descriptor = member.bus.describe(localName);
        out.push({
          name: this.#canonicalName(member, localName),
          localName,
          member: member.id,
          prefix: member.prefix,
          title: descriptor.title,
          description: descriptor.description,
          sideEffects: descriptor.sideEffects,
          reversible: descriptor.reversible ?? true,
        });
      }
    }
    return out.sort((a, b) => a.name.localeCompare(b.name));
  }

  /** Every non-command tool of every app. */
  tools(): MemberTool[] {
    return this.members().flatMap((m) => m.tools ?? []);
  }

  describe(name: string): CommandInfo {
    const { member, localName } = this.#resolve(name);
    const info = this.commands().find((c) => c.member === member.id && c.localName === localName);
    if (!info) throw new NucleusError("not_found", `unknown command: ${name}`);
    return info;
  }

  /** Route one command to its owning app and journal it for unified undo. */
  invoke<R = unknown>(name: string, params: unknown = {}): InvokeOutcome<R> {
    const { member, localName } = this.#resolve(name);
    const descriptor = member.bus.describe(localName);
    const outcome = member.bus.invoke<R>(localName, member.state, params);

    let journalId: number | null = null;
    if (outcome.applied > 0 && descriptor.reversible !== false) {
      this.#ticks += 1;
      const entry: JournalEntry = {
        id: this.#ticks,
        member: member.id,
        prefix: member.prefix,
        command: name,
        localName,
        title: descriptor.title,
      };
      this.#journal.push(entry);
      // A fresh edit invalidates the federation's redo branch.
      this.#redo.length = 0;
      journalId = entry.id;
    }

    return {
      command: name,
      localName,
      member: member.id,
      prefix: member.prefix,
      result: outcome.result,
      applied: outcome.applied,
      docVersion: outcome.docVersion,
      journalId,
    };
  }

  /** Run a non-command tool (file I/O). Not journaled. */
  async invokeTool(name: string, args: Record<string, unknown> = {}): Promise<unknown> {
    const tool = this.tools().find((t) => t.name === name);
    if (!tool) throw new NucleusError("not_found", `unknown tool: ${name}`);
    return tool.run(args);
  }

  /**
   * Undo the last edit, whichever app made it. Entries whose app reports
   * "nothing to undo" (e.g. a manually cleared history) are skipped.
   */
  undo(): UndoOutcome {
    while (this.#journal.length > 0) {
      const entry = this.#journal.pop();
      if (!entry) break;
      const member = this.#memberByPrefix(entry.prefix);
      const response = member.control.handle({ method: "history.undo" });
      if (!response.ok) {
        throw new NucleusError("internal", `undo failed in ${entry.member}: ${response.error?.message ?? "unknown"}`);
      }
      const changed = (response.result as { undone?: boolean } | undefined)?.undone ?? false;
      if (changed) {
        this.#redo.push(entry);
        return { changed: true, entry };
      }
    }
    return { changed: false, reason: "nothing to undo" };
  }

  /** Redo the last undone edit, in the original order. */
  redo(): UndoOutcome {
    while (this.#redo.length > 0) {
      const entry = this.#redo.pop();
      if (!entry) break;
      const member = this.#memberByPrefix(entry.prefix);
      const response = member.control.handle({ method: "history.redo" });
      if (!response.ok) {
        throw new NucleusError("internal", `redo failed in ${entry.member}: ${response.error?.message ?? "unknown"}`);
      }
      const changed = (response.result as { redone?: boolean } | undefined)?.redone ?? false;
      if (changed) {
        this.#journal.push(entry);
        return { changed: true, entry };
      }
    }
    return { changed: false, reason: "nothing to redo" };
  }

  history(): HistoryStatus {
    return {
      canUndo: this.#journal.length > 0,
      canRedo: this.#redo.length > 0,
      depth: this.#journal.length,
      redoDepth: this.#redo.length,
      undo: [...this.#journal],
      redo: [...this.#redo],
    };
  }

  /** Per-app state, ready to feed an agent. */
  state(): MemberState[] {
    return this.members().map((member) => ({
      id: member.id,
      prefix: member.prefix,
      commands: member.bus.list().length,
      tools: (member.tools ?? []).map((t) => t.name),
      docVersion: member.state.docVersion,
      summary: member.summary ? member.summary() : { nodes: Object.keys(member.state.nodes).length },
    }));
  }

  /** `note.create` when unique, `notes/note.create` when two apps collide. */
  #canonicalName(member: Member, localName: string): string {
    const owners = this.members().filter((m) => m.bus.has(localName));
    return owners.length > 1 ? `${member.prefix}/${localName}` : localName;
  }

  #resolve(name: string): { member: Member; localName: string } {
    const slash = name.indexOf("/");
    if (slash > 0) {
      const prefix = name.slice(0, slash);
      const localName = name.slice(slash + 1);
      const member = this.#members.get(prefix);
      if (!member) throw new NucleusError("not_found", `unknown app prefix: ${prefix}`);
      if (!member.bus.has(localName)) throw new NucleusError("not_found", `unknown command: ${name}`);
      return { member, localName };
    }

    const owners = this.members().filter((m) => m.bus.has(name));
    if (owners.length === 0) throw new NucleusError("not_found", `unknown command: ${name}`);
    const first = owners[0];
    if (!first) throw new NucleusError("internal", "unreachable");
    if (owners.length > 1) {
      const qualified = owners.map((m) => `${m.prefix}/${name}`).join(", ");
      throw new NucleusError("validation", `ambiguous command "${name}" — qualify it: ${qualified}`);
    }
    return { member: first, localName: name };
  }

  #memberByPrefix(prefix: string): Member {
    const member = this.#members.get(prefix);
    if (!member) throw new NucleusError("not_found", `unknown app prefix: ${prefix}`);
    return member;
  }
}
