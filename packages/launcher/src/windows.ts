import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { repoRoot } from "./paths.js";

/** Windows browser binaries that support the chromeless `--app=` window mode. */
const CANDIDATES = [
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
];

export function detectBrowser(): string | null {
  for (const candidate of CANDIDATES) {
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

export function detectNucleusShell(): string | null {
  const candidate = join(repoRoot(), "assets", "nucleus-shell.exe");
  return existsSync(candidate) ? candidate : null;
}

export interface AppWindow {
  pid: number | undefined;
  /** Resolves when the window is closed. */
  closed: Promise<void>;
  close(): void;
}

export interface OpenWindowOptions {
  profileDir: string;
  /** Window title (shown on the taskbar and title bar). */
  title?: string;
  /** Window size, e.g. "1440x900". */
  size?: string;
}

/**
 * Opens `url` in its own desktop window. Preferred: the native `nucleus-shell`
 * (WebView2) when it has been built — every nucleus app then gets a real window
 * with an app icon, no browser chrome. Falls back to an isolated Edge/Chrome
 * `--app=` profile. Returns null when neither is available.
 */
export function openAppWindow(url: string, options: OpenWindowOptions): AppWindow | null {
  const shell = detectNucleusShell();
  if (shell) return spawnShell(url, options, shell, join(repoRoot(), "assets", "nucleus.ico"));

  const browser = detectBrowser();
  if (!browser) return null;

  const args = [
    `--app=${url}`,
    `--user-data-dir=${options.profileDir}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-features=Translate,AutofillServerCommunication",
  ];
  if (options.size) args.push(`--window-size=${options.size.replace("x", ",")}`);

  return spawnWindow(browser, args);
}

function spawnShell(
  url: string,
  options: OpenWindowOptions,
  shellPath: string,
  iconPath: string,
): AppWindow {
  const args = [url, "--title", options.title ?? "Nucleus"];
  args.push("--user-data-dir", options.profileDir);
  if (options.size) args.push("--size", options.size);
  if (iconPath) args.push("--icon", iconPath);
  return spawnWindow(shellPath, args);
}

function spawnWindow(binary: string, args: string[]): AppWindow {
  const child = spawn(binary, args, { stdio: "ignore", windowsHide: false });
  const closed = new Promise<void>((resolve) => {
    child.once("exit", () => resolve());
    child.once("error", () => resolve());
  });
  return {
    pid: child.pid,
    closed,
    close: () => {
      try {
        child.kill();
      } catch {
        // already gone
      }
    },
  };
}

/** Last-resort fallback: hand the URL to the default browser. */
export function openDefaultBrowser(url: string): void {
  spawn("cmd", ["/c", "start", "", url], { stdio: "ignore", windowsHide: true });
}
