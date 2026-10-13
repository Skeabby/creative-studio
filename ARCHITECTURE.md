# Nucleus — Architecture & Phased Build Plan

> Status: **Phase 1 built** (2026-10-10). Keep the checkout on local disk so `node_modules/`
> and `.turbo/` never land in a cloud-synced folder.
> Scope: the *nucleus itself* — launcher + shared core + unified control layer + MCP — plus **one** proof app.

> **Update (2026-10-10):** Phase 0 scaffolded and verified green — npm workspaces + Turborepo,
> `@nucleus/core` command bus with 7 passing tests, and a working `CLI → ControlServer → CommandBus`
> spine. Build caches must live outside any cloud-synced folder: npm deletes a `node_modules`
> junction on install, so redirecting caches via a junction is not viable — keeping the checkout on
> local disk resolves risk #1 outright. Risk #2 (rustup) is deferred until Tauri lands (Phase 4).

> **Update #2 (2026-10-10) — naming + launcher pulled earlier.** The deliverable is **"Nucleus"**:
> a personal suite of creative apps in the spirit of ArtCraft Launcher (commercial,
> trademarked — own branding, own app set, no catalog cloning). The launcher (Phase 5 in the table
> below) was **implemented now**, in Node, rather than waiting for Tauri: `apps/launcher` is a
> windowed app (Edge/Chrome `--app=`) listing apps from `nucleus.app.json` manifests, with
> Open/Stop, grid + table views, search and themes. `packages/launcher` owns discovery, manifests,
> process runtime (free port + readiness probe + process-tree kill), window management and the
> `nucleus` CLI. A desktop shortcut makes it launch like a native app. Tauri remains the
> Phase 4 upgrade path for a real always-on shell.
>
> Apps now: **PDF Workbench** (real: `packages/pdf` + `apps/pdf-workbench`) plus the launcher
> itself; 6 further apps (Image, Vector, Video, Audio, Slides, Notes) are declared as *planned*
> entries in `apps/launcher/roadmap.json`.

> **Update #3 (2026-10-10) — native windows, no browser.** The launcher (and every app window)
> no longer opens in Edge/Chrome. `shell/nucleus-shell.cs` is a ~150-line WinForms + **WebView2**
> host compiled with the .NET Framework `csc.exe` (present on every Windows box — no Visual
> Studio, no Rust needed) into `assets/nucleus-shell.exe`. Each app still runs as a local Node
> HTTP server; `packages/launcher` opens it inside a real native window (taskbar identity,
> brand icon, no browser chrome), with Chrome/Edge `--app=` as a fallback. WebView2 runtime is
> preinstalled on Win10/11. Rebuild: `powershell -File scripts\build-shell.ps1` (refetch
> assemblies with `scripts\fetch-webview2.ps1`). Tauri (Phase 4) remains the path to a packaged
> installer + tray; the shell already gives us the native-window behaviour today.

> **Update #4 (2026-10-10) — renamed to "Nucleus" + launcher redesign.** The project was renamed
> from "Creative Studio" to **Nucleus** everywhere: the GitHub repo (`Skeabby/nucleus`), npm scope
> (`@nucleus/*`), CLI (`nucleus`), app ids (`nucleus.*`), manifest file (`nucleus.app.json`), the
> per-user data dir (`%APPDATA%\Nucleus`) and the native shell (`nucleus-shell.exe`). The launcher
> UI was redesigned into a premium app-store look: big icon tiles, status + version, sectioned grid
> (Apps / Coming soon), a prominent Open button, a running dot, and a refined light/dark palette.

> **Update #5 (2026-10-10) — launcher UX, tier 1.** The launcher grew the features that make it feel
> like a real app store: cards are grouped **by category** (Image & Design / Media / Documents /
> System), the table is **sortable** on every column (7 columns incl. inspired-by and source),
> **arrow keys** move a selection and Enter opens, **Ctrl+K** opens a **command palette** that also
> lists each app's command-bus capabilities, and clicking a card opens a **detail drawer** showing
> description, capabilities/permissions chips and the exact `npm run nucleus -- open <id>` command.
> `/api/apps` now also returns `author`, `capabilities`, `permissions`, `source` and `dir`.
> `tools/screenshot.mjs`'s DOM self-check was updated and a second hero (`docs/screenshot-table.png`)
> is captured. Assets (logo, wordmark, app icons) are still pending.

> **Update #6 (2026-10-13) — Notes ships + brand assets.** App #2 is real: **Notes**
> (`apps/notes`, `nucleus.notes`, category `text`, alpha, inspired by OneNote) — a Markdown
> notebook with notebooks/folders, undo/redo and full-text search, running as a local server app
> with `readyPath /api/state`. It was delivered as a full design→build handoff (code + assets),
> then re-verified line-by-line against `@nucleus/core`'s actual API, recompiled under the repo's
> strict TS config and smoke-tested over HTTP before merge. The roadmap now dedupes discovered
> apps via `mergeRoadmap` (`packages/launcher/src/roadmap.ts`), so a real app hides its planned
> card instead of listing twice. Brand assets landed in `docs/brand/` (logo PNG + `nucleus.svg`)
> and `docs/brand/icons/` (7 recolored SVG icons on the token palette: INK `#0B0E14`, SURFACE
> `#151B26`, TEXT `#E7ECF5`, ACCENT `#6D5EF8`, CYAN `#22D3EE`, RUN `#34D399`, AMBER `#F59E0B`);
> every app icon is token-native. **Provenance strip:** the handoff packed C2PA metadata
> (81-89 % of SVG weight + a `caBX` chunk in the PNG) — `tools/strip-c2pa.mjs` removes it from
> SVGs (`<metadata>` + `xmlns:c2pa`) and PNGs (text/`eXIf`/`caBX` chunks, CRCs re-verified) for
> anything committed to the repo. The short TikTok promo clips keep their provenance labels on
> purpose (platform AI-labeling rules) and live outside the public checkout.

**Codename:** `nucleus` · monorepo root `nucleus` · packages `@nucleus/*`

---

## 0. Executive decision

> Build the MVP as a **TypeScript monorepo** (pnpm + Turborepo) with a **Tauri v2** desktop shell,
> the core in TypeScript behind a hard "replaceable native module" seam, and a **single command
> bus** exposed via **CLI + JSON-RPC + MCP**. Target end-state is a Rust-core hybrid; do not start there.

Why:
- No Rust installed + speed-to-MVP dominates → authoring in TS is far faster to first working command.
- Agent-driven development (opencode) iterates on TS/JSON far faster (no slow `cargo build` loop).
- A TS core runs in the browser for free; heavy modules migrate to Rust→WASM later behind the same seam.
- ArtCraft parity is the finish line, not the on-ramp.

---

## 1. Stack recommendation

| | A. Pure Rust (wgpu/egui/iced) | **B. TS monorepo + Tauri v2** | C. Hybrid Rust-core + TS/Web UI |
|---|---|---|---|
| MVP speed | very slow | **fastest** | moderate |
| Native, no Electron | full | WebView2 (not Electron) | full |
| Perf ceiling | best | WebView/WebGPU ceiling | best |
| Toolchain now | needs rustup+cargo | needs rustup (shell only) | needs rustup + napi/wasm |
| Agent iteration | slow compile loop | **hot reload, fast tests** | core loop slow |
| WASM/browser | possible | heavy ops not browser-ready | by construction |
| Risk | high for MVP | **low** | medium (two toolchains + FFI) |

**Verdict:** **B now, C as target, A never as a rewrite.** Tauri still needs the Rust toolchain to
*build* (one-time `rustup`, ~15 min); author zero Rust initially.

**WASM note:** orchestrator, document model, command bus, serialization and most UI logic are
browser-native already (TS). Heavy modules (RAW, video, GPU compositing) go behind the
`Renderer`/`Codec` abstraction so a WASM backend is additive, never an assumption.

---

## 2. Repo structure

```
nucleus/
├─ package.json / pnpm-workspace.yaml / turbo.json / tsconfig.base.json
├─ .github/workflows/ci.yml          # windows-latest primary
├─ .opencode/mcp.json                # register MCP server with opencode
├─ schemas/                          # SINGLE SOURCE OF TRUTH (JSON Schema)
│  ├─ command-descriptor.schema.json
│  ├─ app-manifest.schema.json
│  ├─ document.schema.json
│  ├─ control-protocol.schema.json
│  └─ error.schema.json
├─ packages/
│  ├─ core/          # document model, graph, command bus, undo, serialize
│  ├─ core-native/   # (future) Rust napi/wasm seam — Phase 7
│  ├─ renderer/      # rendering abstraction + backends (cpu/canvas/webgpu)
│  ├─ control/       # ControlServer: transport-agnostic session/invoke/subscribe
│  ├─ control-cli/   # frontend #1: CLI
│  ├─ control-rpc/   # frontend #2: JSON-RPC 2.0 (pipe + WS)
│  ├─ control-mcp/   # frontend #3: MCP server (stdio + HTTP)
│  ├─ launcher/      # discovery/install/update/sign
│  ├─ manifests/     # manifest types, validation, registry client
│  ├─ ui-kit/        # shared UI + design tokens
│  ├─ sdk/           # public SDK for 3rd-party app authors
│  └─ testkit/       # fixtures, golden images, protocol conformance
├─ apps/
│  ├─ pdf-workbench/ # PROOF APP #1 (headless-testable)
│  └─ image-editor/  # PROOF APP #2 (Phase 6, de-risks rendering)
├─ rust/             # cargo workspace — created only in Phase 7
├─ tools/            # codegen, validators, release scripts
├─ examples/         # sample docs, agent scripts, MCP transcripts
└─ docs/             # architecture, protocol, app-authoring guide
```

**Rules:** `schemas/` is the source of truth (codegen TS types, CLI help, MCP tools, docs).
`core` never imports UI/transport. `control-*` never imports app code. Apps depend on `core` + `sdk` only.

---

## 3. Shared core modules (`@nucleus/core`)

Everything flows through commands; **no direct document mutation anywhere else.**

| Module | Responsibility | Key decision |
|---|---|---|
| `document` | root aggregate: metadata, nodes, assets, selection, `docVersion` (CAS) | runtime = flat `Map<NodeId,Node>` + child index |
| `model/graph` | node/layer tree; invariants (acyclic, parent exists, dense order) | tree now, DAG refs later |
| `command` | registry, descriptor lookup, validation, middleware, execution, events | pure handlers `(state, params, ctx) → {patches, events}` |
| `history` | undo/redo | inverse-patch + snapshot checkpoints; entries tagged `{actor, label, requestId}` |
| `serialize` | persistence + versioning | `.nucleus` = ZIP(manifest.json + canonical document.json + content-addressed assets + oplog.jsonl) |
| `render` | rendering abstraction | backends `cpu-headless` (mandatory), `canvas2d`, `webgpu`, later `wgpu` |
| `events` | typed event bus | drives UI, RPC subscriptions, MCP notifications |
| `assets` | content-addressed store | hash-addressed, dedup, integrity |
| `log` | append-only command log (JSONL) | **base doc + command log = deterministic replay** — backbone of undo/MCP/co-editing |

**CPU-first renderer is the most important testability decision** — lets CI and opencode verify
output headlessly (`render --to png --headless`) with zero GUI.

---

## 4. Unified control layer

**One bus, one `ControlServer`, three thin frontends. No business logic in frontends.**

```
CLI ─┐
RPC ─┼─► ControlServer (invoke / describe / list / subscribe / cancel / handshake)
MCP ─┘        │
              ▼
          CommandBus ─► handlers ─► document + history + events
```

- **CLI:** `nucleus serve`, `nucleus cmd <name> --json '<params>'`, `nucleus list|describe`, `nucleus run <script.oplog>`, `nucleus render`.
- **JSON-RPC 2.0 IPC:** Windows named pipe `\\.\pipe\nucleus-<appId>-<pid>` (Node `net`), Unix socket elsewhere, plus loopback WS for browser/debug. Token file (user-only ACL), loopback-only.
- **MCP:** see §5.

**Command descriptor schema** (`schemas/command-descriptor.schema.json`):
`name` (dotted, namespaced), `title`, `description`, `params` (JSON Schema), `result`,
`examples`, `sideEffects` (`read|write|destructive`), `reversible`, `requires` (capabilities),
`since`/`deprecatedIn`/`removedIn` (semver), `mcp` mapping hints, `authorization`.
Registry is self-describing → CLI help + MCP tools + docs are **generated**, zero drift.

**Versioning:** two independent versions — app semver and **control protocol semver** (negotiated at
handshake). Within a major: additive only; renames = add new + deprecate + alias for one major.
Document format versioned separately with explicit migrations.

**Error format** (JSON-RPC envelope, stable machine contract in `data`):
`{ code, message, data: { kind, command, params, details[], retryable,
expectedDocVersion, actualDocVersion }, id }`. Codes `-32000..-32099` reserved for nucleus.
Concurrency conflict → `kind:"conflict"` + versions → agent re-reads and retries (CAS).

**Provenance:** every command carries `actor: { kind: human|agent|system, id, session }` →
audit, per-actor undo, isolated agent sessions.

---

## 5. MCP server (`@nucleus/control-mcp`)

- **Transport:** stdio (primary — opencode launches it) + **Streamable HTTP** (secondary, loopback by default).
- **SDK:** official TypeScript MCP SDK now; Rust `rmcp` later only if the core goes native. Keep the adapter thin (tools → `bus.invoke`).
- **Tool naming:** MCP forbids dots → deterministic mapping `doc.addNode` → `doc_add_node`; `mcp.toolName` can override.
- **Critical decision — do NOT expose every command as a tool.** Expose a curated ~15–30 primary
  tools + a generic escape hatch + discovery tools (progressive disclosure):
  - Primary: `doc_open`, `doc_query`, `doc_apply` (batch), `render_export`, `session_undo`, `session_info`
  - Generic: `cmd_invoke(name, params)`
  - Discovery: `cmd_list`, `cmd_describe(name)`
  - Lifecycle: `app_list`, `app_open`, `app_close`
- **Resources:** current document JSON, selection, command catalog, rendered preview images.
- **Event handling:** stdio is request/response — prefer `session_wait_for_idle` + `doc_query` polling over server push for correctness.

**opencode loop:** register stdio server → list tools → `doc_query` → `doc_open` → `cmd_describe`/`doc_apply` → `render_export` → agent reads returned image → iterate. Human UI talks to the **same core** via RPC → live co-editing, `actor`-tagged, fully auditable via oplog.

---

## 6. Launcher

- **Discovery (priority):** dev `apps/*` (`NUCLEUS_DEV=1`) → bundled → `%APPDATA%\Nucleus\apps\<id>\<version>\` → signed remote catalog.
- **Manifest `nucleus.app.json`:** `id`, `name`, `version`, `icon`, entrypoints, **`coreApiVersion`** (ABI major), `protocolRange`, `permissions`, `capabilities`, `minShell`, `updateChannel`, `integrity` (sha256), `signature` (ed25519).
- **Install/update:** `.nucleusapp` bundle (ZIP) verified (hash + signature) **before** install; **atomic** stage→verify→move→repoint `current` junction; keep prior version for rollback.
- **Core sharing:** launcher ships **one core host**; apps are plugins declaring `coreApiVersion` (major must match). Loader refuses mismatched majors with a clear error.

---

## 7. Proof app: **PDF Workbench**

Chosen because it is **headless-testable** (no GPU), exercises **every** core subsystem
(document model, ordering, command bus, undo, serialize+migrate, oplog replay, RPC, MCP,
error/CAS), is deterministic for golden tests, has real utility, and a permissive license path
(`pdf-lib` MIT, `pdfium` BSD, `pdf.js` Apache — avoid AGPL).

Command surface: `pdf.open`, `pdf.split`, `pdf.merge`, `pdf.rotate`, `pdf.reorder`, `pdf.extract`,
`pdf.redact`, `pdf.metadata`, `pdf.export`, `render.pageToPng`.

**Why not start with the image/vector editor?** It front-loads GPU/rendering risk — exactly what we deferred. Prove control first, then rendering. Image editor is proof #2.

---

## 8. Phased roadmap (person-days, 1 dev + opencode)

| Phase | Name | Deliverables | Exit criteria | Effort |
|---|---|---|---|---|
| **0** | Foundations | repo, pnpm+Turbo, tsconfig, `schemas/` skeleton, CI, `.opencode/mcp.json`, install `rustup` | empty workspace builds+green in CI | 3–5 d |
| **1** | Core MVP | document, graph, command bus, history, serialize (+oplog), CPU render stub, testkit | headless deterministic replayable core; round-trip/migrate tests pass | 10–15 d |
| **2** | Control layer | `control`, `control-rpc` (pipe+WS+token), `control-cli` | drive every core command from CLI end-to-end | 5–8 d |
| **3** | MCP | `control-mcp` (TS SDK, stdio+HTTP), curated tools + `cmd_*`, resources; wire into opencode | **opencode builds a PDF via tools**; transcript committed | 4–6 d |
| **4** | Proof app UI | `apps/pdf-workbench` Tauri v2 shell + minimal UI | native PDF app; **human + agent drive same doc** | 10–15 d |
| **5** | Launcher | manifest, discovery, `.nucleusapp` install/update/rollback, signing, multiplexed MCP | launcher installs/runs PDF Workbench + a stub 2nd app | 7–10 d |
| **6** | Harden + app #2 | image editor proof, WebGPU backend, ABI versioning, perf, public `sdk` | two apps on one core | 15–20 d |
| **7** | Native core (opt.) | Rust napi/wasm hot modules, `wgpu`, `rmcp`, WASM/browser | measurable hot-path win | 20+ d |

**Phases 0–4 → genuinely usable nucleus with one solid proof app: ~6–8 weeks.**

**Milestone gates:** M1 (end P1) core deterministic/replayable · M2 (end P3) agent authors a doc
*without a GUI* · M3 (end P4) human+agent co-edit a native app · M4 (end P5) nucleus becomes a platform.

---

## 9. Key risks

| # | Risk | Sev | Mitigation |
|---|---|---|---|
| 1 | **OneDrive vs build caches** (node_modules/target/.turbo/OneDrive sync storms, locks, corrupt builds) | High | move caches to local disk (junction / env dirs) **or** exclude repo from OneDrive sync — decide in Phase 0 |
| 2 | Tauri needs Rust despite "no Rust" | Med | install `rustup` in Phase 0; minimal `src-tauri`; author zero Rust; fallback Node+WebView2 host |
| 3 | WebView perf ceiling for RAW/video/VFX | Med | heavy ops behind `Renderer`/`Codec` backends; WebGPU; benchmark before committing |
| 4 | headless vs GPU render divergence | Med | CPU renderer for tests only; golden images with tolerance; parity suite in CI |
| 5 | descriptor/protocol/doc drift | High | **generate** TS types, CLI help, MCP tools, docs from `schemas/`; conformance tests |
| 6 | agent tool explosion / context bloat | High | curated tools + `cmd_invoke` + `cmd_describe`; never auto-expose full command set |
| 7 | human/agent concurrent edits clobber | Med | `docVersion` CAS + `kind:"conflict"`; `actor` tagging; isolated sessions; log merge |
| 8 | serialization instability across versions | Med | canonical form + explicit migration chain + fuzz round-trip; never mutate old formats |
| 9 | local IPC security | Med | token file (user-only ACL), loopback-only WS, per-app permissions, destructive confirm, oplog audit |
| 10 | **scope creep (the seven-app temptation)** | High | hard freeze: no app #3+ until M3 |
| 11 | licensing drift (AGPL) | Low | allowlist MIT/BSD/Apache; block AGPL/GPL at dependency review in CI |
| 12 | undo of non-invertible ops | Med | inverse-patch + snapshot checkpoints; `reversible:false` forces checkpoint |
| 13 | named-pipe/WS flakiness on Windows | Low | plain Node `net`; integration tests; token + reconnect |

---

## 10. Decision log

- **D1** TS monorepo + Tauri v2 for MVP (Option B); hybrid (C) as target; pure Rust (A) rejected for MVP.
- **D2** pnpm (Corepack) + Turborepo; npm workspaces fallback; no bun.
- **D3** one command bus; CLI / JSON-RPC (pipe+WS) / MCP as thin frontends.
- **D4** `schemas/` single source of truth → codegen CLI + MCP + docs.
- **D5** curated MCP tools + `cmd_invoke` + discovery; no full command exposure.
- **D6** official TS MCP SDK now; `rmcp` only if core goes native.
- **D7** PDF Workbench = proof #1; image editor = proof #2.
- **D8** flat node table + child index; canonical JSON in ZIP; command `oplog` as replay backbone.
- **D9** launcher ships one core; apps depend on `coreApiVersion` ABI major.
- **D10** resolve OneDrive cache hazard in Phase 0.

---

## 11. Immediate next actions (Phase 0)

1. **Confirm OneDrive cache strategy (junction vs exclude) — blocks everything else.**
2. `corepack enable`; scaffold pnpm + Turborepo.
3. Install `rustup` (Tauri build dependency only).
4. Draft `schemas/command-descriptor.schema.json` + `document.schema.json`.
5. Stand up CI (windows-latest) + an empty core test.

---

**Bottom line:** ship the **control layer first**, in **TypeScript**, with **one bus and three
frontends**, prove it on a **headless PDF Workbench**, and keep a clean seam to migrate hot paths
to Rust later. The differentiator is not the GUI — it is that **an agent and a human drive the same
document through the same command bus**, deterministically and audibly. Build that spine and the
seven apps become plug-ins, not rewrites.
