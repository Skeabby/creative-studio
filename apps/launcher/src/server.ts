#!/usr/bin/env node
/**
 * Nucleus launcher — the window that lists every app in the suite and
 * starts the ones you click.
 */
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { AppRuntime, appRoots, discoverApps, type DiscoveredApp } from "@nucleus/launcher";

const PORT = Number(process.env.PORT ?? 4300);
const LAUNCHER_ID = "nucleus.launcher";
const PUBLIC_DIR = fileURLToPath(new URL("../public", import.meta.url));
const APP_DIR = fileURLToPath(new URL("..", import.meta.url));

const runtime = new AppRuntime({
  onEvent: (event) => process.stdout.write(`[launcher] ${event.type}: ${event.message}\n`),
});

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".json": "application/json; charset=utf-8",
  ".ico": "image/x-icon",
};

interface RoadmapEntry {
  id: string;
  name: string;
  description?: string;
  category?: string;
  inspiredBy?: string;
  emoji?: string;
  color?: string;
}

function sendJson(res: ServerResponse, status: number, body: unknown): void {
  res.writeHead(status, { "content-type": "application/json; charset=utf-8" });
  res.end(JSON.stringify(body));
}

async function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown>> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(chunk as Buffer);
  if (chunks.length === 0) return {};
  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf8")) as Record<string, unknown>;
  } catch {
    return {};
  }
}

async function loadRoadmap(): Promise<RoadmapEntry[]> {
  try {
    const raw = await readFile(join(APP_DIR, "roadmap.json"), "utf8");
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) ? (parsed as RoadmapEntry[]) : [];
  } catch {
    return [];
  }
}

async function listApps(): Promise<unknown[]> {
  const discovered = await discoverApps(appRoots());
  const running = new Map(runtime.list().map((entry) => [entry.id, entry]));

  const real = discovered.map((app) => {
    const { manifest } = app;
    const isSelf = manifest.id === LAUNCHER_ID;
    const live = running.get(manifest.id);
    return {
      id: manifest.id,
      name: manifest.name,
      version: manifest.version,
      description: manifest.description ?? "",
      category: manifest.category ?? "other",
      status: manifest.status ?? "stable",
      inspiredBy: manifest.inspiredBy ?? "",
      iconUrl: manifest.icon ? `/api/apps/${manifest.id}/icon` : null,
      emoji: null,
      color: null,
      source: app.source,
      launchable: Boolean(manifest.launch) && !isSelf,
      running: isSelf || Boolean(live),
      self: isSelf,
      url: live?.url ?? (isSelf ? `http://127.0.0.1:${PORT}` : null),
    };
  });

  const planned = (await loadRoadmap()).map((entry) => ({
    id: entry.id,
    name: entry.name,
    version: "—",
    description: entry.description ?? "",
    category: entry.category ?? "other",
    status: "planned",
    inspiredBy: entry.inspiredBy ?? "",
    iconUrl: null,
    emoji: entry.emoji ?? "✨",
    color: entry.color ?? "#5b8cff",
    source: "roadmap",
    launchable: false,
    running: false,
    self: false,
    url: null,
  }));

  return [...real, ...planned];
}

async function findApp(id: string): Promise<DiscoveredApp | undefined> {
  const apps = await discoverApps(appRoots());
  return apps.find((app) => app.manifest.id === id);
}

async function handleApi(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean> {
  if (url.pathname === "/api/health") {
    sendJson(res, 200, { ok: true, launcher: true, port: PORT });
    return true;
  }

  if (url.pathname === "/api/apps" && req.method === "GET") {
    sendJson(res, 200, { ok: true, apps: await listApps() });
    return true;
  }

  const iconMatch = /^\/api\/apps\/([^/]+)\/icon$/.exec(url.pathname);
  if (iconMatch && req.method === "GET") {
    const id = decodeURIComponent(iconMatch[1] ?? "");
    const app = await findApp(id);
    const icon = app?.manifest.icon;
    if (!app || !icon) {
      sendJson(res, 404, { ok: false, error: { message: "no icon" } });
      return true;
    }
    try {
      const data = await readFile(join(app.dir, icon));
      res.writeHead(200, { "content-type": MIME[extname(icon)] ?? "application/octet-stream" });
      res.end(data);
    } catch {
      sendJson(res, 404, { ok: false, error: { message: "icon unreadable" } });
    }
    return true;
  }

  const actionMatch = /^\/api\/apps\/([^/]+)\/(open|stop)$/.exec(url.pathname);
  if (actionMatch && req.method === "POST") {
    const id = decodeURIComponent(actionMatch[1] ?? "");
    const action = actionMatch[2];

    if (id === LAUNCHER_ID) {
      sendJson(res, 400, { ok: false, error: { message: "the launcher is already open" } });
      return true;
    }

    if (action === "stop") {
      const stopped = await runtime.stop(id);
      sendJson(res, stopped ? 200 : 404, { ok: stopped, apps: await listApps() });
      return true;
    }

    const app = await findApp(id);
    if (!app) {
      sendJson(res, 404, { ok: false, error: { message: `unknown app: ${id}` } });
      return true;
    }
    if (!app.manifest.launch) {
      sendJson(res, 400, { ok: false, error: { message: `${app.manifest.name} is not built yet` } });
      return true;
    }
    try {
      await runtime.launch(app);
      sendJson(res, 200, { ok: true, apps: await listApps() });
    } catch (error) {
      sendJson(res, 500, {
        ok: false,
        error: { message: error instanceof Error ? error.message : String(error) },
        apps: await listApps(),
      });
    }
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
      const url = new URL(req.url ?? "/", `http://127.0.0.1:${PORT}`);
      if (url.pathname.startsWith("/api/")) {
        const handled = await handleApi(req, res, url);
        if (!handled) sendJson(res, 404, { ok: false, error: { message: "unknown endpoint" } });
        return;
      }
      await serveStatic(res, url.pathname);
    } catch (error) {
      sendJson(res, 500, { ok: false, error: { message: error instanceof Error ? error.message : String(error) } });
    }
  })();
});

let shuttingDown = false;
const shutdown = () => {
  if (shuttingDown) return;
  shuttingDown = true;
  void runtime.stopAll().then(() => process.exit(0));
};
process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

server.listen(PORT, "127.0.0.1", () => {
  process.stdout.write(`Nucleus launcher running at http://127.0.0.1:${PORT}\n`);
});
