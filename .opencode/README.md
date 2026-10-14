# opencode integration

`mcp.json` registers two MCP servers so opencode (and any MCP-capable agent) can
drive the same command bus as the CLI and the web apps.

```json
{
  "mcpServers": {
    "nucleus": {
      "type": "local",
      "command": ["node", "packages/control-mcp/dist/index.js"],
      "enabled": true
    },
    "nucleus-federation": {
      "type": "local",
      "command": ["node", "packages/control-federation/dist/mcp.js"],
      "enabled": true
    }
  }
}
```

Build the servers first:

```powershell
npm run build
```

## `nucleus` — one app at a time

| Tool | Purpose |
|------|---------|
| `pdf_load` | Load a PDF (base64) → `{ sourceId, pageCount }` |
| `pdf_add_pages` | Append pages from a source (omit `pages` = all) |
| `pdf_rotate_page` | Rotate a page by relative degrees |
| `pdf_move_page` | Move a page to a new position |
| `pdf_remove_page` | Remove a page |
| `pdf_state` | Read the current page list |
| `cmd_list` | List registered command names |
| `cmd_invoke` | Generic escape hatch for any command |

## `nucleus-federation` — every app in one session

Joins the PDF and Notes apps behind one surface and keeps **one undo stack** for
the whole session. See [`../docs/cross-app.md`](../docs/cross-app.md).

| Tool | Purpose |
|------|---------|
| `apps_list` | Joined apps, with their command and tool counts |
| `cmd_list` | Every command of every app (canonical `app/command` names) |
| `cmd_describe` | Owning app, side effects and reversibility of one command |
| `cmd_invoke` | Invoke any command; it joins the unified undo journal |
| `state_get` | Read every app's state (or one, via `prefix`) |
| `undo_status` | Cross-app undo journal and redo stack |
| `undo` / `redo` | Move through edits whichever app made them |
| `pdf.load`, `pdf.loadFile`, `pdf.export`, `notes.export` | Member tools: byte I/O, not journaled |

Both servers are dependency-free JSON-RPC 2.0 implementations over stdio
(`initialize`, `notifications/initialized`, `tools/list`, `tools/call`).
