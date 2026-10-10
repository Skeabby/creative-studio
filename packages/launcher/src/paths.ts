import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** Repository root, derived from this module's location (packages/launcher/dist). */
export function repoRoot(): string {
  return fileURLToPath(new URL("../../../", import.meta.url));
}

/** Per-user studio data: %APPDATA%\CreativeStudio */
export function studioDataDir(): string {
  const base = process.env["APPDATA"] ?? join(homedir(), ".config");
  return join(base, "CreativeStudio");
}

/** Isolated browser profile so each app window is its own process. */
export function windowProfileDir(appId: string): string {
  const dir = join(studioDataDir(), "window-profiles", appId);
  mkdirSync(dir, { recursive: true });
  return dir;
}

/** Where apps are looked for, in priority order. */
export function appRoots(): string[] {
  return [join(repoRoot(), "apps"), join(studioDataDir(), "apps")];
}
