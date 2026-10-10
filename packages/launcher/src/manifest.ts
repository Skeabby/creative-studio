import { readFile } from "node:fs/promises";
import type { AppLaunchSpec, AppManifest } from "./types.js";

export type ParseResult =
  | { ok: true; manifest: AppManifest }
  | { ok: false; errors: string[] };

const ID_PATTERN = /^[a-z0-9]+(\.[a-z0-9-]+)+$/;

const LAUNCH_KINDS = new Set(["server", "url", "command"]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Hand-written validation for now. When `schemas/` codegen lands this is
 * generated from `app-manifest.schema.json` — the rules here mirror it.
 */
export function parseManifest(value: unknown): ParseResult {
  const errors: string[] = [];
  if (!isRecord(value)) return { ok: false, errors: ["manifest must be a JSON object"] };

  for (const field of ["id", "name", "version", "coreApiVersion"] as const) {
    if (typeof value[field] !== "string" || value[field] === "") {
      errors.push(`missing or invalid required field: ${field}`);
    }
  }

  if (typeof value["id"] === "string" && !ID_PATTERN.test(value["id"])) {
    errors.push(`id must look like "nucleus.app": got "${value["id"]}"`);
  }

  const launch = value["launch"];
  if (launch !== undefined) {
    if (!isRecord(launch)) {
      errors.push("launch must be an object");
    } else {
      const kind = launch["kind"];
      if (typeof kind !== "string" || !LAUNCH_KINDS.has(kind)) {
        errors.push(`launch.kind must be one of: ${[...LAUNCH_KINDS].join(", ")}`);
      } else if (kind === "server") {
        if (!Array.isArray(launch["args"])) errors.push("launch.args is required for kind=server");
      } else if (kind === "url" && typeof launch["url"] !== "string") {
        errors.push("launch.url is required for kind=url");
      }
    }
  }

  if (errors.length > 0) return { ok: false, errors };
  return { ok: true, manifest: value as unknown as AppManifest };
}

export async function loadManifest(manifestPath: string): Promise<ParseResult> {
  let raw: string;
  try {
    raw = await readFile(manifestPath, "utf8");
  } catch (error) {
    return { ok: false, errors: [`cannot read ${manifestPath}: ${String(error)}`] };
  }
  try {
    return parseManifest(JSON.parse(raw));
  } catch (error) {
    return { ok: false, errors: [`${manifestPath} is not valid JSON: ${String(error)}`] };
  }
}

export function launchSpecDefaults(spec: AppLaunchSpec): Required<Pick<AppLaunchSpec, "window" | "readyPath">> {
  return { window: spec.window ?? true, readyPath: spec.readyPath ?? "/" };
}
