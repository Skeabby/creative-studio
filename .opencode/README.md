# opencode integration

`mcp.json` registers the nucleus MCP server so opencode (and any MCP-capable
agent) can drive the same command bus as the CLI and the web app.

```json
{
  "mcpServers": {
    "nucleus": {
      "type": "local",
      "command": ["node", "packages/control-mcp/dist/index.js"],
      "enabled": true
    }
  }
}
```

Build the server first:

```powershell
npm run build
```

## Tools exposed

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

The server is a dependency-free JSON-RPC 2.0 implementation over stdio
(`initialize`, `notifications/initialized`, `tools/list`, `tools/call`).
