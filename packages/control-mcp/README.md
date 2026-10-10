# @nucleus/control-mcp

Frontend **#3** of the control layer (Phase 3).

MCP server that maps tools onto the shared command bus:
- **stdio** (primary — opencode launches it) + **Streamable HTTP** (secondary)
- curated ~15-30 primary tools + generic `cmd_invoke` + `cmd_list` / `cmd_describe`
  (progressive disclosure — never auto-expose the full command set)
- resources: current document JSON, selection, command catalog, rendered previews

Keeps the adapter thin: MCP tool call -> validate -> `bus.invoke`. Not
implemented yet — placeholder for the package layout.
