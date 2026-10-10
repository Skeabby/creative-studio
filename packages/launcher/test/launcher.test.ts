import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { discoverApps, findFreePort, parseManifest, launchSpecDefaults } from "../src/index.js";

const valid = {
  id: "nucleus.demo",
  name: "Demo",
  version: "1.0.0",
  coreApiVersion: "1",
  category: "text",
  status: "stable",
  launch: { kind: "server", args: ["dist/server.js"] },
};

describe("manifest validation", () => {
  it("accepts a well-formed manifest", () => {
    const result = parseManifest(valid);
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.manifest.id).toBe("nucleus.demo");
  });

  it("rejects missing required fields", () => {
    const result = parseManifest({ id: "nucleus.demo" });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.errors.join(" ")).toContain("name");
      expect(result.errors.join(" ")).toContain("version");
    }
  });

  it("rejects a malformed id", () => {
    const result = parseManifest({ ...valid, id: "Demo" });
    expect(result.ok).toBe(false);
  });

  it("requires args for a server launch", () => {
    const result = parseManifest({ ...valid, launch: { kind: "server" } });
    expect(result.ok).toBe(false);
  });

  it("requires a url for a url launch", () => {
    const result = parseManifest({ ...valid, launch: { kind: "url" } });
    expect(result.ok).toBe(false);
  });

  it("rejects unknown launch kinds", () => {
    const result = parseManifest({ ...valid, launch: { kind: "teleport" } });
    expect(result.ok).toBe(false);
  });

  it("defaults window and readyPath", () => {
    expect(launchSpecDefaults({ kind: "server", args: [] })).toEqual({ window: true, readyPath: "/" });
    expect(launchSpecDefaults({ kind: "server", args: [], window: false, readyPath: "/health" })).toEqual({
      window: false,
      readyPath: "/health",
    });
  });
});

describe("discovery", () => {
  let root = "";

  beforeAll(async () => {
    root = await mkdtemp(join(tmpdir(), "nucleus-launcher-"));
    const appsRoot = join(root, "apps");
    await mkdir(join(appsRoot, "alpha"), { recursive: true });
    await mkdir(join(appsRoot, "beta"), { recursive: true });
    await mkdir(join(appsRoot, "not-an-app"), { recursive: true });
    await writeFile(join(appsRoot, "alpha", "nucleus.app.json"), JSON.stringify(valid));
    await writeFile(
      join(appsRoot, "beta", "nucleus.app.json"),
      JSON.stringify({ ...valid, id: "nucleus.beta", name: "Beta" }),
    );
    // A stray file in apps/ and a malformed manifest must not crash discovery.
    await writeFile(join(appsRoot, "strays.txt"), "nope");
    await mkdir(join(appsRoot, "broken"), { recursive: true });
    await writeFile(join(appsRoot, "broken", "nucleus.app.json"), "{ not json");
  });

  afterAll(async () => {
    await rm(root, { recursive: true, force: true });
  });

  it("finds valid apps and skips broken ones", async () => {
    const apps = await discoverApps([join(root, "apps")]);
    expect(apps.map((a) => a.manifest.id)).toEqual(["nucleus.beta", "nucleus.demo"]);
    expect(apps[0]?.dir).toContain("beta");
  });

  it("returns an empty list for a missing root", async () => {
    expect(await discoverApps([join(root, "does-not-exist")])).toEqual([]);
  });

  it("lets an earlier root win on duplicate ids", async () => {
    const second = join(root, "second");
    await mkdir(second, { recursive: true });
    await writeFile(join(second, "nucleus.app.json"), JSON.stringify({ ...valid, name: "Duplicate" }));
    const apps = await discoverApps([second, join(root, "apps")], "test");
    const demo = apps.find((a) => a.manifest.id === "nucleus.demo");
    expect(demo?.manifest.name).toBe("Duplicate");
  });
});

describe("port allocation", () => {
  it("returns a usable port", async () => {
    const port = await findFreePort();
    expect(port).toBeGreaterThan(0);
    expect(port).toBeLessThan(65536);
  });

  it("falls back when the preferred port is taken", async () => {
    const first = await findFreePort();
    const second = await findFreePort(first);
    expect(second).toBeGreaterThan(0);
  });
});
