# Creative Studio

An open, agent-ready studio for creative tools. One shared core, one command bus, three thin
frontends (CLI, JSON-RPC, MCP), and a launcher that hosts apps as plugins.

- Architecture & roadmap: [`ARCHITECTURE.md`](./ARCHITECTURE.md)
- Schemas (single source of truth): [`schemas/`](./schemas)

## Status

**Phase 1 — Launcher + first app live.** The studio has a launcher window listing
every app (Open / Stop, grid + table, search, themes) and its first real app,
**PDF Workbench**: merge, reorder, rotate, delete, undo/redo, save/open projects
and export — driven by the same command bus from the web UI, the CLI, and an MCP server.

**Native windows, not a browser.** Studio apps run in a real desktop window
(`shell/studio-shell.exe` — a small WinForms + WebView2 host compiled with the
.NET Framework `csc.exe`, no installs needed). The launcher window you see is
that shell; so is every app you Open from it.

## Run it

```powershell
npm install
npm run build
npm start              # opens the Creative Studio launcher window
```

Then click **Open** on *PDF Workbench* — it starts on a free port in its own window.
Close the launcher window to shut everything down.

### Desktop shortcut

```powershell
npm run shell          # (one-time) build the native window shell: assets\studio-shell.exe
npm run shortcut       # creates "Creative Studio.lnk" on the desktop (+ an icon)
```

After that you can just double-click **Creative Studio** like any other Windows app.

## Layout

```
packages/core         # document model, command bus, undo/redo, serialization (real)
packages/control      # ControlServer: transport-agnostic invoke/describe/list (real, minimal)
packages/control-cli  # CLI frontend #1 (real, minimal)
packages/control-rpc  # JSON-RPC frontend #2 (Phase 2 — placeholder)
packages/control-mcp  # MCP frontend #3 (real, minimal)
packages/pdf          # @studio/pdf command pack: importPages/rotate/move/remove/export (real)
packages/launcher     # app discovery, app manifests, process runtime, windows, CLI (real)
apps/launcher         # the launcher window: grid/table UI + /api/apps (real)
apps/pdf-workbench    # app #1: local HTTP server + web UI (real)
schemas/              # JSON Schema — source of truth for codegen
tools/                # codegen / validators
scripts/              # desktop entry point + shortcut installer
```

Each app is a folder with a `studio.app.json` manifest describing what it is and
how to launch it — that's all the launcher needs to discover and run it. Windows
are hosted by `shell/studio-shell.cs` (WinForms + WebView2, compiled by
`scripts/build-shell.ps1`).

## Commands

```bash
npm install
npm run verify              # build + typecheck + test (via turbo)
npm run build
npm run test
npm start                   # open the Creative Studio launcher
npm run studio -- list      # list discovered apps
npm run studio -- open studio.pdf-workbench
npm run studio -- serve     # launcher server only, no window
npm run start:pdf           # PDF Workbench directly (http://localhost:5173)
npm run shortcut            # desktop shortcut + icon
npm run mcp                 # run the MCP server over stdio
npm --workspace @studio/control-cli run start -- command.list
```

### MCP

The studio MCP server is registered in [`.opencode/mcp.json`](./.opencode/mcp.json) and exposes
curated PDF tools plus a generic `cmd_invoke` escape hatch — see [`.opencode/README.md`](./.opencode/README.md).

## Location

This repo lives on **local disk** at `C:\Users\<username>\dev\creative-studio`, moved out of OneDrive
so `node_modules/` and `.turbo/` never sync (npm deletes a `node_modules` junction on install, so
redirecting caches via junction is not viable — local disk is). A copy of the design doc stays in
the private notes vault at `projects/creative-studio/ARCHITECTURE.md`.
