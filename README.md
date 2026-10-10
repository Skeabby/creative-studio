# Nucleus

[![CI](https://github.com/Skeabby/nucleus/actions/workflows/ci.yml/badge.svg)](https://github.com/Skeabby/nucleus/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows-0078D4.svg)](#)
[![Node](https://img.shields.io/badge/node-%3E%3D22-43853D.svg)](https://nodejs.org)

> **Nucleus** is a local-first desktop launcher for creative tools. It hosts drop-in apps, every
> app exposes the **same command bus** through CLI, JSON-RPC and MCP frontends, and windows are
> **native** (WebView2) instead of a browser tab — no Electron, no install wizard.

![Nucleus launcher hosting the app suite](docs/screenshot-launcher.png)

---

## Why

Most creative suites are either heavy Electron shells or a dozen disconnected tools. Nucleus
takes a different route:

- **One core, many frontends.** `@nucleus/core` owns the document model, a command bus and
  undo/redo. The web UI, the CLI and an MCP server all drive the *same* commands — anything
  the GUI can do, a script or an AI agent can do too.
- **Apps are plugins.** Drop a folder with a `nucleus.app.json` manifest anywhere the launcher
  scans and it appears in the grid. No registry, no rebuild.
- **Native windows, zero install.** A ~9 KB WinForms + WebView2 host (`shell/nucleus-shell.cs`)
  renders each app in a real desktop window. It compiles with the `csc.exe` already present on
  Windows — nothing to download.
- **A launcher that looks the part.** An app-store style grid: big icons, status and version,
  a clear Open button, plus a sortable table view and light/dark themes.

## Apps

| App | Status | What it does |
|-----|--------|--------------|
| **Nucleus launcher** | 🟠 alpha | Every app in one window: search, grid/table, Open / Stop, themes. |
| **PDF Workbench** | 🟠 beta | Merge, reorder, rotate, delete pages, undo/redo, save/open projects, export. |
| Image Editor | 🗓 planned | Layers, masks, non-destructive adjustments and export. |
| Vector Editor | 🗓 planned | Bézier paths, shapes and vector export. |
| Video Editor | 🗓 planned | Timeline editing, colour and audio mixing. |
| Audio Editor | 🗓 planned | Waveform editing, effects chain and mastering. |
| Slides | 🗓 planned | Decks with layouts, transitions and present mode. |
| Notes | 🗓 planned | Markdown notes, notebooks and full-text search. |

## Quickstart (Windows)

```powershell
npm install
npm run build
npm start          # opens the Nucleus launcher window
```

Click **Open** on *PDF Workbench* — it starts on a free port in its own window. Close the
launcher window to shut the whole nucleus down.

### Make it a real desktop app

```powershell
npm run shell      # (one-time) build the native window shell -> assets\nucleus-shell.exe
npm run shortcut   # create "Nucleus.lnk" on the desktop, with an icon
```

Double-click **Nucleus** like any other Windows app.

## How it works

```
                ┌──────────────────────────── frontends ────────────────────────────┐
                │  launcher UI   CLI (control-cli)   JSON-RPC   MCP (control-mcp)     │
                └───────────────┬───────────────┬──────────────┬──────────────┬───────┘
                                │               │              │              │
                                └───────────────┴──────┬───────┴──────────────┘
                                                       ▼
                                          ControlServer  (transport-agnostic)
                                                       ▼
                              CommandBus  ·  document model  ·  undo/redo  ·  serialization
                                                       ▲
                                    app command packs (e.g. @nucleus/pdf)
```

The **launcher** scans `apps/*/nucleus.app.json` and `%APPDATA%\Nucleus\apps`, picks a
free port per app, spawns `node <app>` detached with a `PORT` env var, waits for a readiness
probe, and opens a native window at the app root. It stays alive so closing the launcher
shuts down the apps it owns.

## Monorepo layout

```
packages/core         # document model, command bus, undo/redo, serialization
packages/control      # ControlServer: invoke / describe / list / history
packages/control-cli  # CLI frontend
packages/control-rpc  # JSON-RPC frontend (Phase 2 placeholder)
packages/control-mcp  # MCP frontend (dependency-free JSON-RPC 2.0 over stdio)
packages/pdf          # @nucleus/pdf command pack (import / rotate / move / remove / export)
packages/launcher     # app discovery, manifests, process runtime, windows, CLI
apps/launcher         # the launcher window (grid/table UI + /api/apps)
apps/pdf-workbench    # app #1: local HTTP server + web UI
schemas/              # JSON Schema - the source of truth for codegen
tools/                # codegen and validators
shell/                # native window host (WinForms + WebView2)
scripts/              # shell build, desktop entry point, shortcut installer
```

Every app is just a folder with a `nucleus.app.json` manifest describing what it is and how to
launch it — that is all the launcher needs to discover and run it.

## Commands

```bash
npm run verify            # build + typecheck + test (via turbo)
npm run build
npm run test
npm start                 # open the Nucleus launcher
npm run nucleus -- list   # list discovered apps
npm run nucleus -- open nucleus.pdf-workbench
npm run nucleus -- serve  # launcher server only, no window
npm run start:pdf         # run PDF Workbench directly
npm run mcp               # run the MCP server over stdio
```

## Agent-ready (MCP)

The nucleus MCP server is registered in [`.opencode/mcp.json`](./.opencode/mcp.json) and exposes
curated PDF tools plus a generic `cmd_invoke` escape hatch — see
[`.opencode/README.md`](./.opencode/README.md). Any MCP-capable agent can drive the same
command bus as the GUI, so the whole suite is scriptable *and* promptable.

## Roadmap

The phased plan, design decisions and risks live in [`ARCHITECTURE.md`](./ARCHITECTURE.md).
Short version: launcher + first app (done) → more apps → JSON-RPC frontend → optional Tauri
packaging for cross-platform native bundles.

## Contributing

PRs and issues are welcome. Run `npm run verify` before opening a PR — it builds, typechecks
and tests every workspace.

## License

[MIT](./LICENSE) © Nucleus contributors.
