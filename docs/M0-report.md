# M0 — Truth & Safety: report

**Date:** 2026-10-17 · **Milestone:** M0 (audit re-verification + fixes)
**Commits:** `4e081e4`, `a4a488c`, `ed6cecc`, `27ea74d` (all on `main`)

## What changed

| Finding | Before | Now |
|---|---|---|
| **F1** Notes sort | `noteViews` compared the wrong way and returned *oldest*-first | newest-first (`b.updatedAt.localeCompare(a.updatedAt)`) |
| **F2** Autosave floods undo | every `note.update` was its own undo entry | `CommandBus` history entries carry `key`+`window`; `note.update` declares `coalesce: { key: (p) => p.id, windowMs: 2000 }`; a burst is one step |
| **F3** XSS in Markdown | text escaped *after* markup insertion; links accepted any scheme | escape-before-markup; link hrefs limited to `http(s)`/`mailto` |
| **F4** Editor clobber | `renderEditor` overwrote the textarea whenever server text differed | `localRev`/`sentRev` generations; server text only adopted when no unsent edits |
| **F5** Bind address | `server.listen(PORT)` (all interfaces) | `server.listen(PORT, "127.0.0.1", …)` in both apps |
| **F6** No request checks | no Host/Origin/Content-Type/token validation | shared `src/guard.ts`: Host allowlist, same-origin, JSON on writes, bearer/`nt`-cookie token; launchers mint + pass the token |
| **F9** README overclaim | "notebooks … full-text search" | README row and ARCHITECTURE Update #12 state only what exists |

## How to verify (repeatable)

Security guard (unit): `npx vitest run` in `apps/notes` — 10 tests: foreign `Origin` → 403, bad
`Host` → 403, `text/plain` POST → 415, missing/wrong token → 401, cookie accepted.

Appendix A repro (F1 + F2), against the built server:

```powershell
cd apps\notes; npx tsc -p tsconfig.json
$env:PORT="5199"; $env:NUCLEUS_TOKEN="repro-token"; node dist/server.js   # in one shell
# in another shell (NUCLEUS_TOKEN=repro-token), POST /api/op twice to create OLD then NEW,
# then five consecutive note.update calls, then drain POST /api/undo.
```

Observed: `titles: ["NEW","OLD"]` (F1 fixed) and `undo steps: 3` (F2 fixed; was 7).

Gates: `npm run verify` (exit 0) and, in `launcher-rs/`, `cargo fmt --check` +
`cargo clippy --all-targets -- -D warnings` + `cargo test --workspace`.

## Tests before / after

| Suite | Before | After |
|---|---|---|
| Rust (`cargo test --workspace`) | 107 (68 bin + 39 core) | 107 (unchanged) |
| TS — core | 10 | **14** |
| TS — notes-app | 0 | **10** |
| TS — total | 43 | **57** |

`docs/scoreboard.md` regenerated; `--check rust` and `--check ts` green.

## Known gaps (tracked, not dropped)

- **F7** Nothing persists — closing a window loses the session. → app-kit / M3 (crash-safe `.nucleus`).
- **F8** Accent drift between app CSS (`--accent`) and launcher category colours. → shared tokens (M?).
- **F10** Core history is anonymous/non-replayable (no actor, no timestamp on the entry, no oplog). → core v2.
- **F11** `command.describe` has no params/result schema. → core v2.
- **F12** `note.create` uses `randomUUID()` + `Date.now()` (non-deterministic). → M1.
- **F13** MCP is PDF-only. → run registry / one MCP (M4).
- **F14** ~200 lines of HTTP boilerplate duplicated across the two apps. → app-kit.
- **F15** PDF Workbench: base64-in-JSON upload, no `pdf.find`, no split/export ZIP. → PDF v0.2.

## Notes

- The token is always required (generated when `NUCLEUS_TOKEN` is unset); standalone dev prints a
  tokenised URL. The launcher passes `?t=` and the app stores it as an `HttpOnly` cookie.
- `guard.ts` is deliberately dependency-free and pure so it can be lifted into `@nucleus/app-kit`.
