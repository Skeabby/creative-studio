/**
 * Member factories — glue between the app command packs and the federation.
 *
 * Each factory owns one app session and exposes:
 *   - its commands (already registered on the app bus),
 *   - a read-only `summary` view,
 *   - the file-I/O tools that cannot be bus commands (load / export).
 */
import { randomUUID } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { ControlServer } from "@nucleus/control";
import { NucleusError } from "@nucleus/core";
import {
  countPages,
  createPdfSession,
  exportPdf,
  pageViews,
  PDF_APP_ID,
  type PdfSession,
} from "@nucleus/pdf";
import {
  createNotesSession,
  exportNotesMarkdown,
  noteViews,
  NOTES_APP_ID,
  type NotesSession,
} from "@nucleus/notes";
import { Federation, type Member } from "./federation.js";

export interface PdfBundle {
  member: Member;
  session: PdfSession;
}

/** The PDF Workbench app, joined into the federation. */
export function pdfMember(): PdfBundle {
  const session = createPdfSession();

  const member: Member = {
    id: PDF_APP_ID,
    prefix: "pdf",
    bus: session.bus,
    state: session.doc,
    control: new ControlServer(session.bus, session.doc),
    summary: () => ({ pages: pageViews(session.doc), sources: session.assets.ids().length }),
    tools: [
      {
        name: "pdf.load",
        description: "Load a PDF from base64 bytes as a source. Returns a sourceId and pageCount.",
        inputSchema: {
          type: "object",
          required: ["dataBase64"],
          properties: { dataBase64: { type: "string" }, name: { type: "string" } },
        },
        run: async (args) => {
          const data = Buffer.from(String(args["dataBase64"] ?? ""), "base64");
          if (data.length === 0) throw new NucleusError("validation", "pdf.load requires dataBase64");
          const sourceId = `src-${randomUUID().slice(0, 8)}`;
          session.assets.put(sourceId, data);
          return { sourceId, pageCount: await countPages(data) };
        },
      },
      {
        name: "pdf.loadFile",
        description: "Load a PDF from a file path on this machine. Returns a sourceId and pageCount.",
        inputSchema: {
          type: "object",
          required: ["path"],
          properties: { path: { type: "string" } },
        },
        run: async (args) => {
          const path = String(args["path"] ?? "");
          if (!path) throw new NucleusError("validation", "pdf.loadFile requires path");
          const data = await readFile(path);
          const sourceId = `src-${randomUUID().slice(0, 8)}`;
          session.assets.put(sourceId, data);
          return { sourceId, pageCount: await countPages(data), byteLength: data.length };
        },
      },
      {
        name: "pdf.export",
        description: "Render the current page list to a real PDF. Optionally write it to a path.",
        inputSchema: {
          type: "object",
          properties: { path: { type: "string" } },
        },
        run: async (args) => {
          const bytes = await exportPdf(session.doc, session.assets);
          const path = typeof args["path"] === "string" ? args["path"] : undefined;
          if (path) await writeFile(path, bytes);
          return {
            byteLength: bytes.length,
            writtenTo: path ?? null,
            dataBase64: Buffer.from(bytes).toString("base64"),
          };
        },
      },
    ],
  };

  return { member, session };
}

export interface NotesBundle {
  member: Member;
  session: NotesSession;
}

/** The Notes app, joined into the federation. */
export function notesMember(): NotesBundle {
  const session = createNotesSession();

  const member: Member = {
    id: NOTES_APP_ID,
    prefix: "notes",
    bus: session.bus,
    state: session.doc,
    control: session.control,
    summary: () => ({ notes: noteViews(session.doc) }),
    tools: [
      {
        name: "notes.export",
        description: "Export every note as one Markdown document. Optionally write it to a path.",
        inputSchema: {
          type: "object",
          properties: { path: { type: "string" } },
        },
        run: async (args) => {
          const markdown = exportNotesMarkdown(session.doc);
          if (markdown.length === 0) throw new NucleusError("validation", "no notes to export");
          const path = typeof args["path"] === "string" ? args["path"] : undefined;
          if (path) await writeFile(path, markdown, "utf8");
          return { bytes: Buffer.byteLength(markdown, "utf8"), writtenTo: path ?? null, markdown };
        },
      },
    ],
  };

  return { member, session };
}

export interface NucleusFederation {
  federation: Federation;
  pdf: PdfBundle;
  notes: NotesBundle;
}

/** Join the app set the POC ships with. */
export function createFederation(): NucleusFederation {
  const pdf = pdfMember();
  const notes = notesMember();
  const federation = new Federation().register(pdf.member).register(notes.member);
  return { federation, pdf, notes };
}
