# @studio/control-rpc

Frontend **#2** of the control layer (Phase 2).

JSON-RPC 2.0 over local IPC:
- Windows named pipe `\\.\pipe\creative-studio-<appId>-<pid>` (Node `net`)
- Unix domain socket on other platforms
- loopback WebSocket for browser / debug tooling

Shares one token file (user-only ACL) and binds WS to loopback only. Methods
mirror `ControlServer`: `command.invoke`, `command.describe`, `command.list`,
`doc.get`, `event.subscribe`, `session.cancel`.

Not implemented yet — this is a placeholder so the package layout is visible.
