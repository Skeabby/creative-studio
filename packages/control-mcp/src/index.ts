#!/usr/bin/env node
/**
 * @studio/control-mcp — frontend #3 of the control layer.
 *
 * A *minimal, dependency-free* MCP server (Model Context Protocol is JSON-RPC
 * 2.0 over stdio). It exposes a curated set of tools plus a generic escape
 * hatch (`cmd_invoke`) so an agent can drive the PDF Workbench.
 */
import { randomUUID } from "node:crypto";
import { createInterface } from "node:readline";
import { countPages, createPdfSession, pageViews } from "@studio/pdf";

const PROTOCOL_VERSION = "2025-06-18";
const session = createPdfSession();

interface ToolDef {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

const TOOLS: ToolDef[] = [
  {
    name: "pdf_load",
    description: "Load a PDF (base64) as a source. Returns a sourceId and pageCount.",
    inputSchema: {
      type: "object",
      required: ["dataBase64"],
      properties: {
        dataBase64: { type: "string" },
        name: { type: "string" },
      },
    },
  },
  {
    name: "pdf_add_pages",
    description: "Append pages from a loaded source to the working document. Omit `pages` to add them all.",
    inputSchema: {
      type: "object",
      required: ["sourceId"],
      properties: {
        sourceId: { type: "string" },
        pages: { type: "array", items: { type: "integer", minimum: 0 } },
      },
    },
  },
  {
    name: "pdf_rotate_page",
    description: "Rotate a page by relative degrees (clockwise).",
    inputSchema: {
      type: "object",
      required: ["id", "degrees"],
      properties: { id: { type: "string" }, degrees: { type: "integer" } },
    },
  },
  {
    name: "pdf_move_page",
    description: "Move a page to a new zero-based position.",
    inputSchema: {
      type: "object",
      required: ["id", "to"],
      properties: { id: { type: "string" }, to: { type: "integer", minimum: 0 } },
    },
  },
  {
    name: "pdf_remove_page",
    description: "Remove a page from the document.",
    inputSchema: { type: "object", required: ["id"], properties: { id: { type: "string" } } },
  },
  {
    name: "pdf_state",
    description: "Read the current page list of the working document.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "cmd_list",
    description: "List every registered command name on the control bus.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "cmd_invoke",
    description: "Generic escape hatch: invoke any command by name.",
    inputSchema: {
      type: "object",
      required: ["name"],
      properties: { name: { type: "string" }, params: { type: "object" } },
    },
  },
];

async function callTool(name: string, args: Record<string, unknown>): Promise<unknown> {
  switch (name) {
    case "pdf_load": {
      const data = Buffer.from(String(args.dataBase64 ?? ""), "base64");
      const sourceId = `mcp-${randomUUID()}`;
      session.assets.put(sourceId, data);
      const pages = await countPages(data);
      return { sourceId, pageCount: pages };
    }
    case "pdf_add_pages": {
      const sourceId = String(args.sourceId);
      let pages = args.pages as number[] | undefined;
      if (!pages) {
        const bytes = session.assets.get(sourceId);
        if (!bytes) throw new Error(`source not found: ${sourceId}`);
        const n = await countPages(bytes);
        pages = Array.from({ length: n }, (_, i) => i);
      }
      return session.bus.invoke("pdf.importPages", session.doc, { sourceId, pages }).result;
    }
    case "pdf_rotate_page":
      return session.bus.invoke("pdf.rotatePage", session.doc, { id: String(args.id), degrees: Number(args.degrees) }).result;
    case "pdf_move_page":
      return session.bus.invoke("pdf.movePage", session.doc, { id: String(args.id), to: Number(args.to) }).result;
    case "pdf_remove_page":
      return session.bus.invoke("pdf.removePage", session.doc, { id: String(args.id) }).result;
    case "pdf_state":
      return { pages: pageViews(session.doc) };
    case "cmd_list":
      return session.bus.list();
    case "cmd_invoke":
      return session.bus.invoke(String(args.name), session.doc, (args.params as object) ?? {}).result;
    default:
      throw new Error(`unknown tool: ${name}`);
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
        serverInfo: { name: "studio-pdf-workbench", version: "0.0.0" },
      },
    });
    return;
  }

  if (method === "notifications/initialized" || method === "notifications/cancelled") return;

  if (method === "tools/list") {
    send({ jsonrpc: "2.0", id, result: { tools: TOOLS } });
    return;
  }

  if (method === "tools/call") {
    const toolName = String(params?.name ?? "");
    const args = (params?.arguments as Record<string, unknown>) ?? {};
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
