import { spawn, type ChildProcess } from "node:child_process";
import { randomBytes } from "node:crypto";
import { findFreePort } from "./ports.js";
import { windowProfileDir } from "./paths.js";
import { launchSpecDefaults } from "./manifest.js";
import { openAppWindow, openDefaultBrowser, type AppWindow } from "./windows.js";
import type { DiscoveredApp } from "./discovery.js";

/** A per-launch secret for an app's HTTP surface (see SPEC security posture). */
function randomToken(): string {
  return randomBytes(16).toString("hex");
}

export interface RunningApp {
  id: string;
  name: string;
  pid: number | undefined;
  port: number | null;
  url: string | null;
  startedAt: string;
}

export type RuntimeEventType = "starting" | "ready" | "stopped" | "failed";

export interface RuntimeEvent {
  type: RuntimeEventType;
  appId: string;
  message: string;
}

export interface AppRuntimeOptions {
  onEvent?: (event: RuntimeEvent) => void;
  readyTimeoutMs?: number;
}

interface Entry {
  running: RunningApp;
  child?: ChildProcess;
  window?: AppWindow | null;
  stopping: boolean;
  closed: Promise<void>;
  resolveClosed: () => void;
}

/** Kills a process and everything it spawned (Windows-safe). */
function killTree(pid: number): Promise<void> {
  return new Promise((resolve) => {
    if (process.platform === "win32") {
      const killer = spawn("taskkill", ["/pid", String(pid), "/T", "/F"], {
        stdio: "ignore",
        windowsHide: true,
      });
      killer.once("exit", () => resolve());
      killer.once("error", () => resolve());
      return;
    }
    try {
      process.kill(pid, "SIGTERM");
    } catch {
      // already gone
    }
    resolve();
  });
}

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Owns the lifecycle of every app the launcher starts: picks a port, spawns the
 * app's server, waits until it answers, opens its window, and cleans up.
 */
export class AppRuntime {
  readonly #entries = new Map<string, Entry>();
  readonly #onEvent: (event: RuntimeEvent) => void;
  readonly #readyTimeoutMs: number;

  constructor(options: AppRuntimeOptions = {}) {
    this.#onEvent = options.onEvent ?? (() => {});
    this.#readyTimeoutMs = options.readyTimeoutMs ?? 20_000;
  }

  list(): RunningApp[] {
    return [...this.#entries.values()].map((entry) => entry.running);
  }

  get(id: string): RunningApp | undefined {
    return this.#entries.get(id)?.running;
  }

  isRunning(id: string): boolean {
    return this.#entries.has(id);
  }

  /** Resolves once the app's window closes (or immediately if it has none). */
  waitForClose(id: string): Promise<void> {
    return this.#entries.get(id)?.closed ?? Promise.resolve();
  }

  async launch(app: DiscoveredApp): Promise<RunningApp> {
    const id = app.manifest.id;
    const existing = this.#entries.get(id);
    if (existing) return existing.running;

    const spec = app.manifest.launch;
    if (!spec) throw new Error(`app "${id}" has no launch spec`);

    let resolveClosed: () => void = () => {};
    const closed = new Promise<void>((resolve) => {
      resolveClosed = resolve;
    });

    const running: RunningApp = {
      id,
      name: app.manifest.name,
      pid: undefined,
      port: null,
      url: null,
      startedAt: new Date().toISOString(),
    };
    const entry: Entry = { running, stopping: false, closed, resolveClosed };

    try {
      this.#onEvent({ type: "starting", appId: id, message: `starting ${app.manifest.name}` });

      if (spec.kind === "command") {
        const command = spec.command ?? process.execPath;
        const child = spawn(command, spec.args ?? [], {
          cwd: app.dir,
          stdio: "ignore",
          windowsHide: true,
          detached: true,
        });
        child.unref();
        entry.child = child;
        running.pid = child.pid;
        child.once("exit", () => this.#forget(id, entry));
        this.#entries.set(id, entry);
        resolveClosed();
        this.#onEvent({ type: "ready", appId: id, message: `${app.manifest.name} launched` });
        return running;
      }

      const { window: wantWindow, readyPath } = launchSpecDefaults(spec);

      let url: string;
      if (spec.kind === "url") {
        url = spec.url ?? "";
      } else {
        const port = await findFreePort(spec.port ?? 0);
        const token = randomToken();
        const command = spec.command ?? process.execPath;
        const child = spawn(command, spec.args ?? [], {
          cwd: app.dir,
          env: { ...process.env, PORT: String(port), NUCLEUS_TOKEN: token },
          stdio: "ignore",
          windowsHide: true,
          detached: true,
        });
        child.unref();
        entry.child = child;
        running.pid = child.pid;
        running.port = port;
        child.once("exit", () => this.#forget(id, entry));
        // readyPath is the health probe; the window always opens the app root.
        await this.#waitReady(port, readyPath, token);
        // The token rides in the URL for the first load; the app swaps it for an
        // HttpOnly cookie and redirects to the clean URL.
        url = `http://127.0.0.1:${port}/?t=${token}`;
      }

      running.url = url;

      if (wantWindow) {
        const appWindow = openAppWindow(url, {
          profileDir: windowProfileDir(id),
          title: app.manifest.name,
          size: "1440x900",
        });
        if (appWindow) {
          entry.window = appWindow;
          void appWindow.closed.then(() => {
            if (!entry.stopping) void this.stop(id);
          });
        } else {
          openDefaultBrowser(url);
          resolveClosed();
        }
      } else {
        resolveClosed();
      }

      this.#entries.set(id, entry);
      this.#onEvent({ type: "ready", appId: id, message: `${app.manifest.name} ready at ${url}` });
      return running;
    } catch (error) {
      if (entry.child?.pid) await killTree(entry.child.pid);
      this.#entries.delete(id);
      const message = error instanceof Error ? error.message : String(error);
      this.#onEvent({ type: "failed", appId: id, message });
      resolveClosed();
      throw new Error(`failed to launch ${id}: ${message}`);
    }
  }

  async stop(id: string): Promise<boolean> {
    const entry = this.#entries.get(id);
    if (!entry) return false;
    entry.stopping = true;
    this.#entries.delete(id);
    entry.window?.close();
    if (entry.child?.pid) await killTree(entry.child.pid);
    entry.resolveClosed();
    this.#onEvent({ type: "stopped", appId: id, message: `${entry.running.name} stopped` });
    return true;
  }

  async stopAll(): Promise<void> {
    await Promise.all([...this.#entries.keys()].map((id) => this.stop(id)));
  }

  async #waitReady(port: number, path: string, token?: string): Promise<void> {
    const deadline = Date.now() + this.#readyTimeoutMs;
    while (Date.now() < deadline) {
      try {
        const response = await fetch(`http://127.0.0.1:${port}${path}`, {
          signal: AbortSignal.timeout(1500),
          headers: token ? { authorization: `Bearer ${token}` } : undefined,
        });
        if (response.ok) return;
      } catch {
        // not up yet
      }
      await delay(250);
    }
    throw new Error(`app did not answer on port ${port} within ${this.#readyTimeoutMs}ms`);
  }

  #forget(id: string, entry: Entry): void {
    if (this.#entries.get(id) === entry) this.#entries.delete(id);
    entry.resolveClosed();
  }
}
