# Cross-app control surface

One agent-facing surface for the whole suite: every app keeps its own command bus,
a **federation** joins them, and the session gets **one undo stack** no matter how
many apps the edits touched.

This is the POC that ships on `poc/cross-app` — the numbers it produces live in
[`scoreboard.md`](./scoreboard.md), not in this prose.

## Shape

```
        ┌─ pdf app  ── CommandBus ─┐
        │                          │        ┌─ control surface ─┐
        ├─ notes app ── CommandBus ─┼───────▶│  Federation       │──▶ MCP server (stdio)
        │                          │        │  one undo journal │    apps_list, cmd_list,
        └─ …next app ── CommandBus ─┘        └───────────────────┘    cmd_describe, cmd_invoke,
                                                                      state_get, undo_status,
                                                                      undo, redo
```

A member is `{ id, prefix, bus, state, control, tools }`. The federation keeps them
in registration order, so `undo` walks the edits back in exactly the order they
happened — across apps.

## Command names

Every command is addressable as `<prefix>.<name>` (`pdf.importPages`, `note.create`).
When two apps expose the same command name, the unqualified name is **rejected** with
a validation error and the caller must qualify it (`notes/note.create`). Silence is
never an option: a wrong guess would edit the wrong app.

## Why tools exist next to commands

Bus commands are synchronous: they take params, return patches, and are journaled.
Loading a PDF, writing an exported file or serialising markdown to disk is I/O, so
those live as **member tools** instead (`pdf.load`, `pdf.loadFile`, `pdf.export`,
`notes.export`). Tools are not journaled — they read or write bytes, they do not
mutate the document model.

## Run it

```bash
npm run mcp:federation     # the aggregated MCP server over stdio (12 tools)
npm run demo:cross-app     # replayable transcript: PDF + Notes in one session
npm run demo:cross-app -- --export   # …and read the exported PDF back
npm run scoreboard         # refresh the measured numbers
```

The demo is the honest demo: it drives real commands, prints a PASS/FAIL line per
claim, and exits non-zero if any of them breaks. It runs inside `npm run verify`, so
CI reproduces the claim on every push.

## What the transcript proves

- 4 edits across 2 apps (import PDF pages, rotate a page, create a note, edit it)
  are undone by 4 `undo` calls, in reverse order, then restored by 4 `redo` calls.
- Read-only commands (`doc.stats`) never enter the journal.
- The exported PDF reopens with 3 pages and page 1 still rotated 90°.

## Not claimed yet

- **Permissions**: `nucleus.app.json` declares `permissions`, but the federation does
  not enforce them yet — the assistant can currently reach every command.
- **`control-rpc`**: the JSON-RPC frontend does not exist; `packages/control-rpc/`
  holds a README only. README and ARCHITECTURE say so explicitly.
- **Network transports**: the federation speaks stdio MCP only (no HTTP/WS).
- **PNG export / PDF text extraction**: the Assistant demo promised in the v0.4 plan
  needs those; neither is implemented, so neither is claimed.
- **The <60 s demo video**: to be recorded once the two items above exist.
