#!/usr/bin/env node
/**
 * `creative-studio` — the launcher CLI.
 *
 *   creative-studio list          show every discovered app
 *   creative-studio open <id>     launch an app and leave it running
 *   creative-studio up            launch the launcher window (this is what the
 *                                 desktop shortcut runs)
 *   creative-studio serve         run the launcher server without a window
 */
import { discoverApps, type DiscoveredApp } from "./discovery.js";
import { appRoots, studioDataDir } from "./paths.js";
import { AppRuntime } from "./runtime.js";

export const LAUNCHER_APP_ID = "studio.launcher";

const HELP = `creative-studio — launcher

Usage:
  creative-studio list             List discovered apps
  creative-studio open <app-id>    Launch an app and leave it running
  creative-studio up               Open the Creative Studio launcher window
  creative-studio serve [--port]   Run the launcher server only (no window)

App roots:
  ${appRoots().join("\n  ")}

Data:
  ${studioDataDir()}
`;

function print(message: string): void {
  process.stdout.write(`${message}\n`);
}

async function resolveApp(id: string): Promise<DiscoveredApp> {
  const apps = await discoverApps(appRoots());
  const app = apps.find((candidate) => candidate.manifest.id === id);
  if (!app) {
    throw new Error(`unknown app "${id}". Run: creative-studio list`);
  }
  return app;
}

async function main(argv: string[]): Promise<number> {
  const [command = "help", ...rest] = argv;
  const runtime = new AppRuntime({
    onEvent: (event) => print(`[${event.type}] ${event.message}`),
  });

  switch (command) {
    case "list": {
      const apps = await discoverApps(appRoots());
      if (apps.length === 0) {
        print("No apps found.");
        return 0;
      }
      const width = Math.max(...apps.map((a) => a.manifest.id.length));
      for (const app of apps) {
        const { id, name, version, status } = app.manifest;
        print(`${id.padEnd(width)}  ${name}  v${version}  ${status ?? "unknown"}`);
      }
      return 0;
    }

    case "open": {
      const id = rest[0];
      if (!id) {
        print("usage: creative-studio open <app-id>");
        return 2;
      }
      const app = await resolveApp(id);
      const running = await runtime.launch(app);
      print(`Started ${running.name}${running.url ? ` at ${running.url}` : ""}`);
      return 0;
    }

    case "up": {
      const app = await resolveApp(LAUNCHER_APP_ID);
      const running = await runtime.launch(app);
      print(`Creative Studio is running at ${running.url}`);
      print("Close the launcher window to shut the studio down.");
      await runtime.waitForClose(LAUNCHER_APP_ID);
      await runtime.stopAll();
      return 0;
    }

    case "serve": {
      const portFlag = rest.indexOf("--port");
      const port = portFlag >= 0 ? Number(rest[portFlag + 1]) : 0;
      const base = await resolveApp(LAUNCHER_APP_ID);
      const app: DiscoveredApp = {
        ...base,
        manifest: {
          ...base.manifest,
          launch: { ...base.manifest.launch, kind: "server", window: false, ...(port ? { port } : {}) },
        },
      };
      const running = await runtime.launch(app);
      print(`Launcher server on ${running.url} (Ctrl+C to stop)`);
      const shutdown = () => {
        void runtime.stopAll().then(() => process.exit(0));
      };
      process.on("SIGINT", shutdown);
      process.on("SIGTERM", shutdown);
      await new Promise(() => {});
      return 0;
    }

    case "help":
    case "--help":
    case "-h":
      print(HELP);
      return 0;

    default:
      print(`Unknown command: ${command}\n`);
      print(HELP);
      return 2;
  }
}

main(process.argv.slice(2))
  .then((code) => process.exit(code))
  .catch((error: unknown) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exit(1);
  });
