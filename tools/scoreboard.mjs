#!/usr/bin/env node
/**
 * Regenerates docs/scoreboard.md from real command output.
 *
 * Every number in that file comes from a command run right here — nothing is
 * typed by hand, which is what makes it a claim instead of a wish.
 *
 *   npm run scoreboard                 regenerate both sections (TS + Rust)
 *   node tools/scoreboard.mjs --check ts    fail if the TS section is stale   (CI)
 *   node tools/scoreboard.mjs --check rust  fail if the Rust section is stale (CI)
 *
 * Sections are delimited by `<!-- section:name --> ... <!-- /section:name -->`,
 * so each CI workflow owns exactly one of them.
 */
import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const file = join(root, "docs", "scoreboard.md");

/** Strip ANSI colour codes (turbo/vitest/cargo colourise their output). */
const strip = (text) => text.replace(/\u001b\[[0-9;]*m/g, "");

function run(command, cwd = root) {
  // `2>&1` keeps stdout and stderr interleaved in real order — cargo prints
  // "Running unittests ..." on stderr and "test result: ..." on stdout, and the
  // crate name has to be paired with its own count.
  const result = spawnSync(`${command} 2>&1`, {
    cwd,
    encoding: "utf8",
    shell: true,
    maxBuffer: 64 * 1024 * 1024,
    timeout: 20 * 60 * 1000,
  });
  const text = strip(`${result.stdout ?? ""}\n${result.stderr ?? ""}`);
  if (result.status !== 0) {
    process.stderr.write(`scoreboard: command failed: ${command}\n`);
    process.stderr.write(`${text.split("\n").slice(-40).join("\n")}\n`);
    process.exit(1);
  }
  return text;
}

// ---------------------------------------------------------------- TypeScript

function measureTs() {
  const verify = run("npm run verify");
  const tasks = /Tasks:\s+(\d+) successful, (\d+) total/.exec(verify);
  if (!tasks) {
    process.stderr.write("scoreboard: could not read the turbo summary out of `npm run verify`\n");
    process.exit(1);
  }

  const tests = [...verify.matchAll(/(@nucleus\/[\w-]+):test:.*Tests\s+(\d+) passed/gm)]
    .map((match) => [match[1].replace("@nucleus/", ""), Number(match[2])])
    // turbo runs packages in parallel: sort so the file is identical on every machine.
    .sort(([a], [b]) => a.localeCompare(b));
  const testTotal = tests.reduce((sum, [, count]) => sum + count, 0);
  const testDetail = tests.map(([name, count]) => `${name} ${count}`).join(" · ");

  const demo = run("npm run demo:cross-app -- --export");
  const passed = (demo.match(/^\s*PASS/gm) ?? []).length;
  const failed = (demo.match(/^\s*FAIL/gm) ?? []).length;
  const commands = Number(/commands\s*:\s*(\d+) unified/.exec(demo)?.[1] ?? 0);
  const edits = Number(/edits to unwind\s*:\s*(\d+)/.exec(demo)?.[1] ?? 0);
  const exportedPages = Number(/(\d+) pages read back/.exec(demo)?.[1] ?? 0);
  const apps = (demo.match(/^apps\s*:\s*(.+)$/m) ?? [])[1]?.trim() ?? "unknown";
  if (failed > 0) {
    process.stderr.write(`scoreboard: demo:cross-app reported ${failed} failing check(s)\n`);
    process.exit(1);
  }
  if (edits === 0 || exportedPages === 0) {
    process.stderr.write("scoreboard: could not read the demo metrics (edits / exported pages)\n");
    process.exit(1);
  }

  const tools = federationTools();
  const packages = countWorkspaces(join(root, "packages"));
  const appsDirs = countWorkspaces(join(root, "apps"));

  return [
    "### TypeScript",
    "",
    "| Claim | Measured | Reproduce |",
    "|---|---|---|",
    `| Build, typecheck and test tasks pass | **${tasks[1]} / ${tasks[2]}** | \`npm run verify\` |`,
    `| TypeScript unit tests | **${testTotal}** (${testDetail}) | \`npm run test\` |`,
    `| Apps joined into one session → unified commands | **${commands} commands from ${countApps(apps)} apps** (${apps}) | \`npm run demo:cross-app\` |`,
    `| One undo path across those apps: ${edits} edits in ${countApps(apps)} apps, unwound then replayed | **${passed} / ${passed} checks passed** | \`npm run demo:cross-app\` |`,
    `| Export from the edited state is a readable PDF | **${exportedPages} pages read back** | \`npm run demo:cross-app -- --export\` |`,
    `| MCP tools on the federation server | **${tools.length}** (${tools.join(", ")}) | \`npm run mcp:federation\` then \`tools/list\` |`,
    `| Workspace packages / apps | **${packages} / ${appsDirs}** | \`npm run scoreboard\` |`,
    "",
    "**Not claimed yet** (honesty rule — no number, no claim): PNG export of a PDF page, PDF text",
    "extraction, assistant permissions enforced from the manifest, a scripted demo video under 60 s.",
    "Those land with v0.4; today's proof stops where the checks stop.",
  ].join("\n");
}

/** One MCP round-trip against the real server: initialize + tools/list. */
function federationTools() {
  const request = [
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}',
    '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}',
    "",
  ].join("\n");
  const server = spawnSync("node", ["packages/control-federation/dist/mcp.js"], {
    cwd: root,
    encoding: "utf8",
    input: request,
    timeout: 30000,
  });
  const names = [];
  for (const line of strip(server.stdout ?? "").split("\n")) {
    if (!line.trim()) continue;
    try {
      const message = JSON.parse(line);
      if (message.id === 2) {
        for (const tool of message.result?.tools ?? []) names.push(tool.name);
      }
    } catch {
      /* ignore anything that is not a JSON-RPC frame */
    }
  }
  if (names.length === 0) {
    process.stderr.write("scoreboard: the federation MCP server did not answer tools/list\n");
    process.exit(1);
  }
  return names;
}

const countApps = (names) => names.split("·").length;
/** A workspace is a directory npm actually installs: one that has a package.json. */
const countWorkspaces = (dir) =>
  existsSync(dir) ? readdirSync(dir).filter((name) => existsSync(join(dir, name, "package.json"))).length : 0;

// --------------------------------------------------------------------- Rust

function measureRust() {
  const cargo = spawnSync("cargo", ["--version"], { encoding: "utf8", shell: true });
  if (cargo.status !== 0) return null;

  const output = run("cargo test --workspace", join(root, "launcher-rs"));
  const counts = [];
  let current = null;
  for (const line of output.split("\n")) {
    const running = /Running unittests .*[\\/]deps[\\/]([\w-]+?)-[0-9a-f]+\.exe/.exec(line);
    if (running) current = running[1];
    const result = /test result: ok\. (\d+) passed/.exec(line);
    if (result) {
      counts.push([current ?? "doctests", Number(result[1])]);
      current = null;
    }
  }
  const total = counts.reduce((sum, [, count]) => sum + count, 0);
  if (total === 0) {
    process.stderr.write("scoreboard: no Rust tests found in `cargo test --workspace` output\n");
    process.exit(1);
  }
  const detail = counts
    .filter(([, count]) => count > 0)
    .map(([name, count]) => `${name} ${count}`)
    .join(" · ");

  return [
    "### Rust (`launcher-rs/`)",
    "",
    "| Claim | Measured | Reproduce |",
    "|---|---|---|",
    `| Rust unit tests | **${total}** (${detail}) | \`cargo test --workspace\` |`,
    "",
    "The `Launcher (Rust)` workflow runs fmt, clippy (\`-D warnings\`), tests and a release build on",
    "every change under `launcher-rs/`; this section is diffed against it there.",
  ].join("\n");
}

// --------------------------------------------------------------- file layout

const header = `# Scoreboard — measured claims

Every number below is produced by a command in this repository. None is typed by
hand: that is the difference between a claim and a measurement.

- **Regenerate**: \`npm run scoreboard\` (TypeScript + cross-app + Rust when cargo is available).
- **CI fails when this file is stale**: \`npm run scoreboard -- --check ts\` in \`.github/workflows/ci.yml\`,
  \`node tools/scoreboard.mjs --check rust\` in \`.github/workflows/launcher-rs.yml\`.
- Claimed elsewhere (README, ARCHITECTURE) numbers must match this file.

`;

function sectionOf(text, name) {
  const match = new RegExp(`<!-- section:${name} -->\\s*([\\s\\S]*?)\\s*<!-- /section:${name} -->`).exec(text);
  return match ? match[1] : null;
}

function assemble(existing, sections) {
  const base = existing ?? `${header}<!-- section:ts -->\n\n<!-- /section:ts -->\n\n<!-- section:rust -->\n\n<!-- /section:rust -->\n`;
  let out = base;
  for (const [name, body] of Object.entries(sections)) {
    if (body === null) continue;
    const pattern = new RegExp(`(<!-- section:${name} -->)[\\s\\S]*?(<!-- /section:${name} -->)`);
    out = pattern.test(out)
      ? out.replace(pattern, `$1\n\n${body}\n\n$2`)
      : `${out.trimEnd()}\n\n<!-- section:${name} -->\n\n${body}\n\n<!-- /section:${name} -->\n`;
  }
  return out;
}

function firstDifference(expected, actual) {
  const a = (actual ?? "").split("\n");
  const b = expected.split("\n");
  for (let i = 0; i < Math.max(a.length, b.length); i += 1) {
    if (a[i] !== b[i]) return `  section line ${i + 1}:\n    file:     ${a[i] ?? "(missing)"}\n    measured: ${b[i] ?? "(missing)"}`;
  }
  return "";
}

// --------------------------------------------------------------------- main

const argv = process.argv.slice(2);
const checkIndex = argv.indexOf("--check");
const which = checkIndex === -1 ? null : argv[checkIndex + 1] ?? "all";

const existing = existsSync(file) ? readFileSync(file, "utf8") : null;

const sections = {
  ts: which === null || which === "all" || which === "ts" ? measureTs() : null,
  rust: which === null || which === "all" || which === "rust" ? measureRust() : null,
};

if (which === null) {
  writeFileSync(file, assemble(existing, sections), "utf8");
  const ts = sections.ts.split("\n")[0];
  process.stdout.write(`scoreboard: ${file}\n  ${ts}\n  ${sections.rust.split("\n")[0] ?? "(cargo unavailable: Rust section left untouched)"}\n`);
  process.exit(0);
}

let failed = false;
for (const [name, body] of Object.entries(sections)) {
  if (body === null) continue;
  const current = sectionOf(existing ?? "", name);
  if (current === body) {
    process.stdout.write(`scoreboard: section "${name}" is up to date\n`);
  } else {
    process.stderr.write(`scoreboard: section "${name}" is STALE — regenerate with \`npm run scoreboard\`\n${firstDifference(current, body)}\n`);
    failed = true;
  }
}
process.exit(failed ? 1 : 0);
