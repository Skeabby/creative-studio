#!/usr/bin/env node
/**
 * Notes — Markdown note-taking app on the Nucleus command bus.
 * Same server pattern as PDF Workbench: vanilla node:http, no framework.
 */
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import type { DocumentState } from "@nucleus/core";
import {
  createNotesSession,
  exportNotesMarkdown,
  noteViews,
  NOTES_PROJECT_FORMAT,
  type NotesSession,
} from "@nucleus/notes";

const PORT = Number(process.env.PORT ?? 5174);
const PUBLIC_DIR = fileURLToPath(new URL("../public", import.meta.url));

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

let session: NotesSession = createNotesSession();

// ---------------------------------------------------------------------------
// Views
// ---------------------------------------------------------------------------

const snapshot = () => ({
  notes: noteViews(session.doc),
  docVersion: session.doc.docVersion,
  canUndo: session.bus.canUndo(),
  canRedo: session.bus.canRedo(),
});

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// API handlers
// ---------------------------------------------------------------------------

async function handleApi(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean> {
  // Health / initial state
  if (url.pathname === "/api/state" && req.method === "GET") {
    sendJson(res, 200, { ok: true, ...snapshot() });
    return true;
  }

  // Generic command invoke (used by the UI for note.create, note.update, note.delete)
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
    session = createNotesSession();
    sendJson(res, 200, { ok: true, ...snapshot() });
    return true;
  }

  // Export all notes as a single Markdown file
  if (url.pathname === "/api/export" && req.method === "GET") {
    const views = noteViews(session.doc);
    if (views.length === 0) {
      sendJson(res, 400, { ok: false, error: { kind: "validation", message: "no notes to export" } });
      return true;
    }
    const md = exportNotesMarkdown(session.doc);
    res.writeHead(200, {
      "content-type": "text/markdown; charset=utf-8",
      "content-disposition": 'attachment; filename="notes.md"',
    });
    res.end(md);
    return true;
  }

  // Save project
  if (url.pathname === "/api/project" && req.method === "GET") {
    const project = {
      format: NOTES_PROJECT_FORMAT,
      version: 1,
      savedAt: new Date().toISOString(),
      doc: session.doc,
    };
    res.writeHead(200, {
      "content-type": "application/json; charset=utf-8",
      "content-disposition": 'attachment; filename="notes.nucleus.json"',
    });
    res.end(JSON.stringify(project));
    return true;
  }

  // Load project
  if (url.pathname === "/api/project" && req.method === "POST") {
    const body = await readJson(req);
    if (body["format"] !== NOTES_PROJECT_FORMAT) {
      sendJson(res, 400, { ok: false, error: { kind: "validation", message: "not a nucleus notes project" } });
      return true;
    }
    const restored = createNotesSession();
    const doc = body["doc"] as Partial<DocumentState> | undefined;
    if (!doc || typeof doc !== "object") {
      sendJson(res, 400, { ok: false, error: { kind: "validation", message: "project has no document" } });
      return true;
    }
    restored.doc.nodes = doc.nodes ?? {};
    restored.doc.selection = doc.selection ?? [];
    restored.doc.docVersion = doc.docVersion ?? 0;
    session = restored;
    sendJson(res, 200, { ok: true, ...snapshot() });
    return true;
  }

  return false;
}

// ---------------------------------------------------------------------------
// Static file server
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

const server = createServer((req, res) => {
  void (async () => {
    try {
      const url = new URL(req.url ?? "/", `http://localhost:${PORT}`);
      if (url.pathname.startsWith("/api/")) {
        const handled = await handleApi(req, res, url);
        if (!handled) {
          sendJson(res, 404, { ok: false, error: { kind: "not_found", message: "unknown endpoint" } });
        }
        return;
      }
      await serveStatic(res, url.pathname);
    } catch (error) {
      sendJson(res, 500, {
        ok: false,
        error: { kind: "internal", message: error instanceof Error ? error.message : String(error) },
      });
    }
  })();
});

server.listen(PORT, () => {
  process.stdout.write(`Notes running at http://localhost:${PORT}\n`);
});
