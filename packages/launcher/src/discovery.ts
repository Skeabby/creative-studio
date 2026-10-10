import { readdir, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import { loadManifest } from "./manifest.js";
import type { AppManifest } from "./types.js";

export interface DiscoveredApp {
  manifest: AppManifest;
  /** Absolute directory containing the app. */
  dir: string;
  manifestPath: string;
  source: string;
}

async function isDirectory(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isDirectory();
  } catch {
    return false;
  }
}

/** Loads `<dir>/studio.app.json`, or null when absent/invalid. */
export async function loadApp(dir: string, source: string): Promise<DiscoveredApp | null> {
  const manifestPath = join(dir, "studio.app.json");
  const result = await loadManifest(manifestPath);
  if (!result.ok) return null;
  return { manifest: result.manifest, dir: resolve(dir), manifestPath, source };
}

/**
 * Scans each root for apps: the root itself, plus every child directory.
 * Later duplicates of an id are ignored, so earlier roots win.
 */
export async function discoverApps(roots: string[], source = "dev"): Promise<DiscoveredApp[]> {
  const found = new Map<string, DiscoveredApp>();

  for (const root of roots) {
    const candidates: string[] = [root];
    try {
      const entries = await readdir(root, { withFileTypes: true });
      for (const entry of entries) {
        if (entry.isDirectory()) candidates.push(join(root, entry.name));
      }
    } catch {
      // Root missing — still try the root itself below.
    }

    for (const dir of candidates) {
      if (!(await isDirectory(dir))) continue;
      const app = await loadApp(dir, source);
      if (app && !found.has(app.manifest.id)) found.set(app.manifest.id, app);
    }
  }

  return [...found.values()].sort((a, b) => a.manifest.name.localeCompare(b.manifest.name));
}
