import { CommandBus, createDocument, NucleusError } from "@nucleus/core";
import { ControlServer } from "@nucleus/control";
import { PDFDocument } from "pdf-lib";
import { describe, expect, it } from "vitest";
import { createFederation, Federation, notesMember, pdfMember, type Member } from "../src/index.js";

async function makePdf(pages: number): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  for (let i = 0; i < pages; i += 1) doc.addPage([300, 400]);
  return doc.save();
}

/** Load an N-page fixture into the federated PDF app via its real tool. */
async function loadFixture(federation: Federation, pages: number): Promise<string> {
  const bytes = await makePdf(pages);
  const loaded = (await federation.invokeTool("pdf.load", {
    dataBase64: Buffer.from(bytes).toString("base64"),
  })) as { sourceId: string };
  return loaded.sourceId;
}

function notesCount(federation: Federation): number {
  const state = federation.state().find((s) => s.prefix === "notes");
  return (state?.summary as { notes: unknown[] }).notes.length;
}

function pdfPages(federation: Federation): { id: string; rotation: number }[] {
  const state = federation.state().find((s) => s.prefix === "pdf");
  return (state?.summary as { pages: { id: string; rotation: number }[] }).pages;
}

describe("Federation — unified surface over app command buses", () => {
  it("lists every command of every joined app", () => {
    const { federation } = createFederation();
    const names = federation.commands().map((c) => c.name);
    expect(names).toContain("pdf.importPages");
    expect(names).toContain("note.create");
    expect(names).toContain("doc.stats");
    expect(federation.prefixes().sort()).toEqual(["notes", "pdf"]);
  });

  it("routes a command to its owning app and journals it", () => {
    const { federation } = createFederation();
    const outcome = federation.invoke("note.create", { title: "A", content: "B" });
    expect(outcome.member).toBe("nucleus.notes");
    expect(outcome.applied).toBe(1);
    expect(outcome.journalId).not.toBeNull();
    expect(notesCount(federation)).toBe(1);
  });

  it("does not journal read-only commands", () => {
    const { federation } = createFederation();
    const outcome = federation.invoke("doc.stats", {});
    expect(outcome.applied).toBe(0);
    expect(outcome.journalId).toBeNull();
    expect(federation.history().depth).toBe(0);
  });

  it("undoes edits across two apps, in reverse order", async () => {
    const { federation } = createFederation();
    const sourceId = await loadFixture(federation, 3);

    const imported = federation.invoke<{ ids: string[] }>("pdf.importPages", { sourceId, pages: [0, 1, 2] });
    const pageId = imported.result.ids[0] ?? "";
    federation.invoke("pdf.rotatePage", { id: pageId, degrees: 90 });
    federation.invoke("note.create", { title: "Review", content: "from the PDF" });

    expect(pdfPages(federation)).toHaveLength(3);
    expect(pdfPages(federation)[0]?.rotation).toBe(90);
    expect(notesCount(federation)).toBe(1);

    // The journal must interleave apps — that is the whole point.
    expect(federation.history().undo.map((e) => e.prefix)).toEqual(["pdf", "pdf", "notes"]);

    expect(federation.undo().entry?.prefix).toBe("notes");
    expect(notesCount(federation)).toBe(0);
    expect(federation.undo().entry?.prefix).toBe("pdf");
    expect(pdfPages(federation)[0]?.rotation).toBe(0);
    expect(federation.undo().entry?.prefix).toBe("pdf");
    expect(pdfPages(federation)).toHaveLength(0);
    expect(federation.undo().changed).toBe(false);

    // …and redo replays them in the original order.
    expect(federation.redo().entry?.prefix).toBe("pdf");
    expect(pdfPages(federation)).toHaveLength(3);
    expect(federation.redo().entry?.prefix).toBe("pdf");
    expect(pdfPages(federation)[0]?.rotation).toBe(90);
    expect(federation.redo().entry?.prefix).toBe("notes");
    expect(notesCount(federation)).toBe(1);
    expect(federation.history().depth).toBe(3);
    expect(federation.history().redoDepth).toBe(0);
  });

  it("a fresh edit drops the redo branch", () => {
    const { federation } = createFederation();
    federation.invoke("note.create", { title: "A" });
    federation.undo();
    expect(federation.history().canRedo).toBe(true);
    federation.invoke("note.create", { title: "B" });
    expect(federation.history().canRedo).toBe(false);
  });

  it("refuses an ambiguous command and accepts the qualified form", () => {
    const notes = notesMember();
    const other = stubMember("other", "note.create");
    const federation = new Federation().register(notes.member).register(other);

    let error: unknown;
    try {
      federation.invoke("note.create", {});
    } catch (caught) {
      error = caught;
    }
    expect(error).toBeInstanceOf(NucleusError);
    expect((error as NucleusError).kind).toBe("validation");
    expect((error as NucleusError).message).toContain("notes/note.create");

    expect(federation.invoke("notes/note.create", { title: "ok" }).member).toBe("nucleus.notes");
    expect(federation.invoke("other/note.create", {}).member).toBe("nucleus.other");
  });

  it("reports unknown commands and unknown apps", () => {
    const { federation } = createFederation();
    expect(() => federation.invoke("nope.missing")).toThrowError(/unknown command/);
    expect(() => federation.invoke("ghost/doc.stats")).toThrowError(/unknown app prefix/);
  });

  it("routes tools and rejects unknown ones", async () => {
    const { federation } = createFederation();
    federation.invoke("note.create", { title: "Exportable", content: "hello" });
    const exported = (await federation.invokeTool("notes.export", {})) as { markdown: string };
    expect(exported.markdown).toContain("# Exportable");
    await expect(federation.invokeTool("ghost.tool", {})).rejects.toThrowError(/unknown tool/);
  });

  it("rejects a duplicate prefix and a duplicate member id", () => {
    const a = pdfMember();
    const b = pdfMember();
    const federation = new Federation().register(a.member);
    expect(() => federation.register(b.member)).toThrowError(/prefix already registered/);
  });

  it("exposes each app's summary for the agent", () => {
    const { federation } = createFederation();
    const states = federation.state();
    expect(states.map((s) => s.prefix)).toEqual(["pdf", "notes"]);
    expect(states.every((s) => s.commands > 0)).toBe(true);
    expect(states[0]?.tools).toContain("pdf.export");
    expect(states[1]?.tools).toContain("notes.export");
  });
});

/** A minimal second app that deliberately collides on one command name. */
function stubMember(prefix: string, command: string): Member {
  const bus = new CommandBus();
  const state = createDocument(`nucleus.${prefix}`);
  bus.register({
    name: command,
    title: "Stub",
    description: "Collision probe.",
    sideEffects: "write",
    reversible: true,
    run: () => ({ patches: [], result: { ok: true } }),
  });
  return {
    id: `nucleus.${prefix}`,
    prefix,
    bus,
    state,
    control: new ControlServer(bus, state),
  };
}
