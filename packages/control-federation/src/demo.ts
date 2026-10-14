#!/usr/bin/env node
/**
 * `npm run demo:cross-app` — the POC transcript, replayed with real commands.
 *
 * Scenario (no mocks, no scripted UI): an agent edits the PDF Workbench AND the
 * Notes app in one session, then unwinds BOTH with a single unified undo path.
 */
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PDFDocument } from "pdf-lib";
import { createFederation, type NucleusFederation } from "./members.js";

const nucleus: NucleusFederation = createFederation();
const { federation } = nucleus;

const failures: string[] = [];
const out = (line = ""): void => { process.stdout.write(`${line}\n`); };

function check(label: string, condition: boolean): void {
  out(`   ${condition ? "PASS" : "FAIL"}  ${label}`);
  if (!condition) failures.push(label);
}

function step(n: number, title: string): void {
  out("");
  out(`${n}) ${title}`);
}

/** Read one app's read-only summary the same way an agent would. */
function summary<T>(prefix: string): T {
  const state = federation.state().find((s) => s.prefix === prefix);
  return state?.summary as T;
}

async function makeFixture(pages: number): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  for (let i = 0; i < pages; i += 1) doc.addPage([595, 842]);
  return doc.save();
}

async function main(): Promise<void> {
  out("Nucleus — cross-app POC (federated commands + unified undo)");
  out("=".repeat(58));
  out(`apps     : ${federation.members().map((m) => `${m.id} (${m.prefix})`).join(" · ")}`);
  out(`commands : ${federation.commands().length} unified`);
  out(`tools    : ${federation.tools().map((t) => t.name).join(", ")}`);

  // 1 — load a real PDF
  const fixture = await makeFixture(3);
  const loaded = (await federation.invokeTool("pdf.load", {
    dataBase64: Buffer.from(fixture).toString("base64"),
    name: "fixture.pdf",
  })) as { sourceId: string; pageCount: number };
  step(1, "pdf.load (agent)");
  out(`   → sourceId=${loaded.sourceId} pageCount=${loaded.pageCount}`);

  // 2 — import the pages through the unified command bus
  const imported = federation.invoke<{ ids: string[] }>("pdf.importPages", {
    sourceId: loaded.sourceId,
    pages: [0, 1, 2],
  });
  step(2, "cmd_invoke pdf.importPages");
  out(`   → ${imported.applied} page(s) · journal #${imported.journalId} · doc v${imported.docVersion}`);

  // 3 — rotate page 1 (PDF app)
  const firstPage = imported.result.ids[0] ?? "";
  const rotated = federation.invoke<{ rotation: number }>("pdf.rotatePage", { id: firstPage, degrees: 90 });
  step(3, "cmd_invoke pdf.rotatePage");
  out(`   → rotation=${rotated.result.rotation}° · journal #${rotated.journalId}`);

  // 4 — create a note that summarises the PDF (other app!)
  const pdfPages = summary<{ pages: { id: string; order: number; rotation: number }[] }>("pdf").pages;
  const review = [
    `# PDF review`,
    ``,
    `- pages: ${pdfPages.length}`,
    `- rotations: ${pdfPages.map((p) => p.rotation).join(", ")}`,
    ``,
    `_Written by the agent from live PDF state._`,
  ].join("\n");
  const note = federation.invoke<{ id: string }>("note.create", { title: "PDF review", content: review });
  step(4, "cmd_invoke note.create  ⟵ different app");
  out(`   → note ${note.result.id} · journal #${note.journalId}`);

  // 5 — edit that note
  const updated = federation.invoke("note.update", {
    id: note.result.id,
    content: `${review}\n\n_Reviewed._`,
  });
  step(5, "cmd_invoke note.update");
  out(`   → journal #${updated.journalId}`);

  const before = {
    pages: summary<{ pages: unknown[] }>("pdf").pages.length,
    notes: summary<{ notes: unknown[] }>("notes").notes.length,
  };
  const edits = federation.history().depth;
  out("");
  out(`state after 5 commands : pdf.pages=${before.pages} · notes=${before.notes}`);
  out(`journal                : ${federation.history().undo.map((e) => e.prefix).join(" ← ")}`);
  out(`edits to unwind        : ${edits} (read-only commands are never journaled)`);

  // 6 — one undo path, two apps
  step(6, `undo × ${edits} (unified — mixed apps)`);
  for (let i = 0; i < edits; i += 1) {
    const undone = federation.undo();
    out(`   undo ${i + 1} → ${undone.changed ? `${undone.entry?.prefix}: ${undone.entry?.title}` : "nothing"}`);
  }

  const afterUndo = {
    pages: summary<{ pages: unknown[] }>("pdf").pages.length,
    notes: summary<{ notes: unknown[] }>("notes").notes.length,
  };
  out("");
  check("PDF back to 0 pages", afterUndo.pages === 0);
  check("Notes back to 0 notes", afterUndo.notes === 0);
  check("undo journal empty", federation.history().depth === 0);
  check(`redo stack holds ${edits} entries`, federation.history().redoDepth === edits);

  // 7 — redo everything back
  step(7, `redo × ${edits} (restores the whole session)`);
  for (let i = 0; i < edits; i += 1) {
    const redone = federation.redo();
    out(`   redo ${i + 1} → ${redone.changed ? `${redone.entry?.prefix}: ${redone.entry?.title}` : "nothing"}`);
  }

  const afterRedo = {
    pages: summary<{ pages: { rotation: number }[] }>("pdf").pages,
    notes: summary<{ notes: unknown[] }>("notes").notes.length,
  };
  out("");
  check("PDF restored to 3 pages", afterRedo.pages.length === 3);
  check("page 1 still rotated 90°", afterRedo.pages[0]?.rotation === 90);
  check("note restored", afterRedo.notes === 1);

  // 8 — export a real PDF from the edited state
  if (process.argv.includes("--export")) {
    const target = join(tmpdir(), "nucleus-cross-app-demo.pdf");
    step(8, `pdf.export → ${target}`);
    const exported = (await federation.invokeTool("pdf.export", { path: target })) as {
      byteLength: number;
      writtenTo: string;
    };
    out(`   → ${exported.byteLength} bytes written to ${exported.writtenTo}`);
    check("exported PDF is non-trivial", exported.byteLength > 500);
  }

  out("");
  out("=".repeat(58));
  if (failures.length === 0) {
    out("RESULT: all checks passed — cross-app commands + unified undo work.");
  } else {
    out(`RESULT: ${failures.length} check(s) FAILED:`);
    for (const failure of failures) out(`  - ${failure}`);
    process.exitCode = 1;
  }
}

main().catch((error: unknown) => {
  out(`demo crashed: ${error instanceof Error ? error.stack ?? error.message : String(error)}`);
  process.exitCode = 1;
});
