#!/usr/bin/env node
/**
 * Federated MCP frontend — the agent-facing surface of the whole suite.
 *
 * One MCP server joins every Nucleus app: the agent lists apps, invokes any
 * command by name (`pdf.importPages`, `note.create`, …) and undoes across apps
 * with a single `undo` call.
 *
 * Dependency-free JSON-RPC 2.0 over stdio, same shape as `@nucleus/control-mcp`.
 */
import { createInterface } from "node:readline";
import { createFederation, type NucleusFederation } from "./members.js";

const PROTOCOL_VERSION = "2025-06-18";
const nucleus: NucleusFederation = createFederation();
const { federation } = nucleus;

interface ToolDef {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

const CORE_TOOLS: ToolDef[] = [
  {
    name: "apps_list",
    description: "List the joined apps, with their command and tool counts.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "cmd_list",
    description: "List every command of every app (canonical invocation names).",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "cmd_describe",
    description: "Describe one command: owning app, side effects, reversibility.",
    inputSchema: { type: "object", required: ["name"], properties: { name: { type: "string" } } },
  },
  {
    name: "cmd_invoke",
    description: "Invoke any command by name — routes to its app and joins the unified undo journal.",
    inputSchema: {
      type: "object",
      required: ["name"],
      properties: { name: { type: "string" }, params: { type: "object" } },
    },
  },
  {
    name: "state_get",
    description: "Read the state of every app (or one app via `prefix`).",
    inputSchema: { type: "object", properties: { prefix: { type: "string" } } },
  },
  {
    name: "undo_status",
    description: "Show the cross-app undo journal and redo stack.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "undo",
    description: "Undo the last edit — whichever app made it, across the whole suite.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "redo",
    description: "Redo the last undone edit, in the original order.",
    inputSchema: { type: "object", properties: {} },
  },
];

function toolDefs(): ToolDef[] {
  return [
    ...CORE_TOOLS,
    ...federation.tools().map((tool) => ({
      name: tool.name,
      description: tool.description,
      inputSchema: tool.inputSchema,
    })),
  ];
}

async function callTool(name: string, args: Record<string, unknown>): Promise<unknown> {
  switch (name) {
    case "apps_list":
      return federation.members().map((member) => ({
        id: member.id,
        prefix: member.prefix,
        commands: member.bus.list().length,
        tools: (member.tools ?? []).map((t) => t.name),
      }));
    case "cmd_list":
      return federation.commands();
    case "cmd_describe":
      return federation.describe(String(args["name"] ?? ""));
    case "cmd_invoke":
      return federation.invoke(String(args["name"] ?? ""), args["params"] ?? {});
    case "state_get": {
      const prefix = typeof args["prefix"] === "string" ? args["prefix"] : undefined;
      const all = federation.state();
      return prefix ? all.filter((s) => s.prefix === prefix || s.id === prefix) : all;
    }
    case "undo_status":
      return federation.history();
    case "undo":
      return federation.undo();
    case "redo":
      return federation.redo();
    default:
      return federation.invokeTool(name, args);
  }
}

function send(message: unknown): void {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

async function handle(message: { id?: unknown; method?: string; params?: Record<string, unknown> }): Promise<void> {
  const { id, method, params } = message;

  if (method === "initialize") {
    send({
      jsonrpc: "2.0",
      id,
      result: {
        protocolVersion: PROTOCOL_VERSION,
        capabilities: { tools: {} },
        serverInfo: { name: "nucleus-federation", version: "0.0.0" },
      },
    });
    return;
  }

  if (method === "notifications/initialized" || method === "notifications/cancelled") return;

  if (method === "tools/list") {
    send({ jsonrpc: "2.0", id, result: { tools: toolDefs() } });
    return;
  }

  if (method === "tools/call") {
    const toolName = String(params?.["name"] ?? "");
    const args = (params?.["arguments"] as Record<string, unknown>) ?? {};
    try {
      const result = await callTool(toolName, args);
      send({
        jsonrpc: "2.0",
        id,
        result: { content: [{ type: "text", text: JSON.stringify(result) }] },
      });
    } catch (error) {
      send({
        jsonrpc: "2.0",
        id,
        result: {
          content: [{ type: "text", text: error instanceof Error ? error.message : String(error) }],
          isError: true,
        },
      });
    }
    return;
  }

  if (id !== undefined && id !== null) {
    send({ jsonrpc: "2.0", id, error: { code: -32601, message: `method not found: ${method}` } });
  }
}

const rl = createInterface({ input: process.stdin });
rl.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  try {
    void handle(JSON.parse(trimmed));
  } catch (error) {
    process.stderr.write(`mcp: invalid message: ${error instanceof Error ? error.message : String(error)}\n`);
  }
});
