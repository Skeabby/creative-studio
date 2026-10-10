import type { CommandBus } from "./command.js";
import type { Node } from "./document.js";
import { StudioError } from "./errors.js";

export interface CoreCommandOptions {
  /** Injectable id factory (deterministic for tests). */
  createId?: () => string;
}

let counter = 0;
function defaultId(): string {
  counter += 1;
  return `node-${counter.toString(36)}`;
}

/** Registers the baseline document commands every app inherits. */
export function registerCoreCommands(bus: CommandBus, options: CoreCommandOptions = {}): void {
  const createId = options.createId ?? defaultId;

  bus.register<{ type: string; parentId?: string | null; props?: Record<string, unknown> }, { id: string }>({
    name: "doc.addNode",
    title: "Add node",
    description: "Add a node to the document and return its id.",
    sideEffects: "write",
    reversible: true,
    run(params, ctx) {
      const id = createId();
      const node: Node = {
        id,
        type: params.type,
        parentId: params.parentId ?? null,
        order: Object.keys(ctx.state.nodes).length,
        enabled: true,
        opacity: 1,
        props: params.props ?? {},
      };
      return { patches: [{ op: "add", nodeId: id, after: node }], result: { id } };
    },
  });

  bus.register<{ id: string; props: Record<string, unknown> }, { id: string }>({
    name: "doc.updateNodeProps",
    title: "Update node props",
    description: "Shallow-merge properties into an existing node.",
    sideEffects: "write",
    reversible: true,
    run(params, ctx) {
      const node = ctx.state.nodes[params.id];
      if (!node) throw new StudioError("not_found", `node not found: ${params.id}`);
      const after: Node = { ...node, props: { ...node.props, ...params.props } };
      return { patches: [{ op: "update", nodeId: params.id, after }], result: { id: params.id } };
    },
  });

  bus.register<{ id: string }, { id: string }>({
    name: "doc.removeNode",
    title: "Remove node",
    description: "Remove a node from the document.",
    sideEffects: "destructive",
    reversible: true,
    run(params, ctx) {
      if (!ctx.state.nodes[params.id]) {
        throw new StudioError("not_found", `node not found: ${params.id}`);
      }
      return { patches: [{ op: "remove", nodeId: params.id }], result: { id: params.id } };
    },
  });

  bus.register<Record<string, never>, { nodes: number; docVersion: number }>({
    name: "doc.stats",
    title: "Document stats",
    description: "Read-only summary of the document.",
    sideEffects: "read",
    reversible: false,
    run(_params, ctx) {
      return {
        result: { nodes: Object.keys(ctx.state.nodes).length, docVersion: ctx.state.docVersion },
      };
    },
  });
}
