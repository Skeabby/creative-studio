import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** Repository root, derived from this module's location (packages/launcher/dist). */
export function repoRoot(): string {
  return fileURLToPath(new URL("../../../", import.meta.url));
}

/** Per-user nucleus data: %APPDATA%\Nucleus */
export function nucleusDataDir(): string {
  const base = process.env["APPDATA"] ?? join(homedir(), ".config");
  return join(base, "Nucleus");
}

/** Isolated browser profile so each app window is its own process. */
export function windowProfileDir(appId: string): string {
  const dir = join(nucleusDataDir(), "window-profiles", appId);
  mkdirSync(dir, { recursive: true });
  return dir;
}

/** Where apps are looked for, in priority order. */
export function appRoots(): string[] {
  return [join(repoRoot(), "apps"), join(nucleusDataDir(), "apps")];
}
