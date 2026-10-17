#!/usr/bin/env node
/**
 * PDF Workbench — a small local app that drives the same command bus as the
 * CLI and MCP server. Serves a static UI and a thin HTTP control surface.
 */
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { readFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { ControlServer } from "@nucleus/control";
import { countPages, createPdfSession, exportPdf, pageViews } from "@nucleus/pdf";
import { guard as guardRequest } from "./guard.js";

const NUCLEUS_TOKEN = process.env.NUCLEUS_TOKEN ?? Math.random().toString(36).slice(2);

const PORT = Number(process.env.PORT ?? 5173);
const PUBLIC_DIR = fileURLToPath(new URL("../public", import.meta.url));
const PROJECT_FORMAT = "nucleus.project";

interface Session {
  assets: ReturnType<typeof createPdfSession>["assets"];
  doc: ReturnType<typeof createPdfSession>["doc"];
  bus: ReturnType<typeof createPdfSession>["bus"];
  control: ControlServer;
}

function newSession(): Session {
  const base = createPdfSession();
  return { ...base, control: new ControlServer(base.bus, base.doc) };
}

let session: Session = newSession();

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
};

function sendJson(res: ServerResponse, status: number, body: unknown): void {
  res.writeHead(status, { "content-type": "application/json; charset=utf-8" });
  res.end(JSON.stringify(body));
}

async function readJson(req: IncomingMessage): Promise<Record<string, unknown>> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(chunk as Buffer);
  if (chunks.length === 0) return {};
  return JSON.parse(Buffer.concat(chunks).toString("utf8")) as Record<string, unknown>;
}

const snapshot = () => ({
  pages: pageViews(session.doc),
  docVersion: session.doc.docVersion,
  canUndo: session.bus.canUndo(),
  canRedo: session.bus.canRedo(),
});

async function handleApi(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean> {
  if (url.pathname === "/api/state" && req.method === "GET") {
    sendJson(res, 200, { ok: true, ...snapshot() });
    return true;
  }

  // Raw source bytes, so the browser can render thumbnails with pdf.js.
  if (url.pathname.startsWith("/api/source/") && req.method === "GET") {
    const id = decodeURIComponent(url.pathname.slice("/api/source/".length));
    const bytes = session.assets.get(id);
    if (!bytes) {
      sendJson(res, 404, { ok: false, error: { kind: "not_found", message: `source not found: ${id}` } });
      return true;
    }
    res.writeHead(200, { "content-type": "application/pdf", "cache-control": "no-store" });
    res.end(Buffer.from(bytes));
    return true;
  }

  if (url.pathname === "/api/load" && req.method === "POST") {
    const body = await readJson(req);
    const bytes = Buffer.from(String(body.dataBase64 ?? ""), "base64");
    if (bytes.length === 0) {
      sendJson(res, 400, { ok: false, error: { kind: "validation", message: "empty upload" } });
      return true;
    }
    const sourceId = `src-${randomUUID()}`;
    session.assets.put(sourceId, bytes);
    const pages = await countPages(bytes);
    sendJson(res, 200, { ok: true, sourceId, pageCount: pages });
    return true;
  }

  if (url.pathname === "/api/op" && req.method === "POST") {
    const body = await readJson(req);
    const response = session.control.handle({ method: "command.invoke", params: body });
    if (!response.ok) {
      sendJson(res, 400, { ok: false, error: response.error, ...snapshot() });
      return true;
    }
    const result = (response.result as { result?: unknown } | undefined)?.result;
    sendJson(res, 200, { ok: true, result, ...snapshot() });
    return true;
  }

  if (url.pathname === "/api/undo" && req.method === "POST") {
    const response = session.control.handle({ method: "history.undo" });
    const undone = (response.result as { undone?: boolean } | undefined)?.undone ?? false;
    sendJson(res, 200, { ok: true, undone, ...snapshot() });
    return true;
  }

  if (url.pathname === "/api/redo" && req.method === "POST") {
    const response = session.control.handle({ method: "history.redo" });
    const redone = (response.result as { redone?: boolean } | undefined)?.redone ?? false;
    sendJson(res, 200, { ok: true, redone, ...snapshot() });
    return true;
  }

  if (url.pathname === "/api/reset" && req.method === "POST") {
    session = newSession();
    sendJson(res, 200, { ok: true, ...snapshot() });
    return true;
  }

  if (url.pathname === "/api/export" && req.method === "GET") {
    const bytes = await exportPdf(session.doc, session.assets);
    if (bytes.length === 0) {
      sendJson(res, 400, { ok: false, error: { kind: "validation", message: "document is empty" } });
      return true;
    }
    res.writeHead(200, {
      "content-type": "application/pdf",
      "content-disposition": 'attachment; filename="workbench.pdf"',
    });
    res.end(Buffer.from(bytes));
    return true;
  }

  // Save the whole project (document + source files) as a portable JSON file.
  if (url.pathname === "/api/project" && req.method === "GET") {
    const assets: Record<string, string> = {};
    for (const id of session.assets.ids()) {
      const bytes = session.assets.get(id);
      if (bytes) assets[id] = Buffer.from(bytes).toString("base64");
    }
    const project = {
      format: PROJECT_FORMAT,
      version: 1,
      app: session.doc.appId,
      savedAt: new Date().toISOString(),
      doc: session.doc,
      assets,
    };
    res.writeHead(200, {
      "content-type": "application/json; charset=utf-8",
      "content-disposition": 'attachment; filename="workbench.nucleus.json"',
    });
    res.end(JSON.stringify(project));
    return true;
  }

  if (url.pathname === "/api/project" && req.method === "POST") {
    const body = await readJson(req);
    if (body.format !== PROJECT_FORMAT) {
      sendJson(res, 400, { ok: false, error: { kind: "validation", message: "not a nucleus project file" } });
      return true;
    }
    const restored = newSession();
    const assets = (body.assets ?? {}) as Record<string, string>;
    for (const [id, b64] of Object.entries(assets)) {
      restored.assets.put(id, Buffer.from(b64, "base64"));
    }
    const doc = body.doc as typeof restored.doc | undefined;
    if (!doc || typeof doc !== "object") {
      sendJson(res, 400, { ok: false, error: { kind: "validation", message: "project has no document" } });
      return true;
    }
    restored.doc.nodes = doc.nodes ?? {};
    restored.doc.selection = doc.selection ?? [];
    restored.doc.docVersion = doc.docVersion ?? 0;
    restored.doc.schemaVersion = doc.schemaVersion ?? 1;
    session = restored;
    sendJson(res, 200, { ok: true, ...snapshot() });
    return true;
  }

  return false;
}

async function serveStatic(res: ServerResponse, pathname: string): Promise<void> {
  const rel = pathname === "/" ? "index.html" : pathname.replace(/^\/+/, "");
  const target = normalize(join(PUBLIC_DIR, rel));
  if (!target.startsWith(PUBLIC_DIR)) {
    res.writeHead(403);
    res.end("forbidden");
    return;
  }
  try {
    const data = await readFile(target);
    res.writeHead(200, { "content-type": MIME[extname(target)] ?? "application/octet-stream" });
    res.end(data);
  } catch {
    res.writeHead(404, { "content-type": "text/plain" });
    res.end("not found");
  }
}

const server = createServer((req, res) => {
  void (async () => {
    try {
      const url = new URL(req.url ?? "/", `http://localhost:${PORT}`);
      if (url.searchParams.get("t") === NUCLEUS_TOKEN) {
        res.writeHead(302, {
          "set-cookie": `nt=${NUCLEUS_TOKEN}; HttpOnly; SameSite=Strict; Path=/`,
          location: url.pathname,
        });
        res.end();
        return;
      }
      if (!guardRequest(req, res, { port: PORT, token: NUCLEUS_TOKEN })) return;
      if (url.pathname.startsWith("/api/")) {
        const handled = await handleApi(req, res, url);
        if (!handled) sendJson(res, 404, { ok: false, error: { kind: "not_found", message: "unknown endpoint" } });
        return;
      }
      await serveStatic(res, url.pathname);
    } catch (error) {
      sendJson(res, 500, { ok: false, error: { kind: "internal", message: error instanceof Error ? error.message : String(error) } });
    }
  })();
});

server.listen(PORT, "127.0.0.1", () => {
  process.stdout.write(`PDF Workbench running at http://localhost:${PORT}/?t=${NUCLEUS_TOKEN}\n`);
});
