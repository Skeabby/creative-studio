import { describe, expect, it } from "vitest";
import { CommandBus, createDocument, registerCoreCommands, StudioError } from "../src/index.js";

function setup() {
  const doc = createDocument("test.app");
  const bus = new CommandBus({ kind: "agent", id: "opencode" });
  registerCoreCommands(bus, { createId: () => `n${Object.keys(doc.nodes).length + 1}` });
  return { doc, bus };
}

describe("@studio/core command bus", () => {
  it("registers and lists commands", () => {
    const { bus } = setup();
    expect(bus.list()).toEqual([
      "doc.addNode",
      "doc.removeNode",
      "doc.stats",
      "doc.updateNodeProps",
    ]);
  });

  it("invokes a command, returns a structured result and bumps docVersion", () => {
    const { doc, bus } = setup();
    const res = bus.invoke<{ id: string }>("doc.addNode", doc, { type: "layer" });
    expect(res.result.id).toBe("n1");
    expect(res.applied).toBe(1);
    expect(doc.nodes["n1"]?.type).toBe("layer");
    expect(doc.docVersion).toBe(1);
  });

  it("undoes a reversible command", () => {
    const { doc, bus } = setup();
    bus.invoke("doc.addNode", doc, { type: "layer" });
    expect(bus.canUndo()).toBe(true);
    expect(bus.undo(doc)).toBe(true);
    expect(Object.keys(doc.nodes)).toHaveLength(0);
  });

  it("throws a typed StudioError for unknown commands", () => {
    const { doc, bus } = setup();
    try {
      bus.invoke("nope.nope", doc);
      throw new Error("expected throw");
    } catch (error) {
      expect(error).toBeInstanceOf(StudioError);
      expect((error as StudioError).kind).toBe("not_found");
    }
  });

  it("does not record history for read-only commands", () => {
    const { doc, bus } = setup();
    const res = bus.invoke<{ nodes: number }>("doc.stats", doc);
    expect(res.result.nodes).toBe(0);
    expect(bus.canUndo()).toBe(false);
    expect(doc.docVersion).toBe(0);
  });

  it("supports dry-run without mutating", () => {
    const { doc, bus } = setup();
    const res = bus.invoke("doc.addNode", doc, { type: "layer" }, { dryRun: true });
    expect(res.applied).toBe(1);
    expect(Object.keys(doc.nodes)).toHaveLength(0);
    expect(doc.docVersion).toBe(0);
  });

  it("updates node props by merging", () => {
    const { doc, bus } = setup();
    bus.invoke("doc.addNode", doc, { type: "layer", props: { fill: "red" } });
    bus.invoke("doc.updateNodeProps", doc, { id: "n1", props: { opacity: 0.5 } });
    expect(doc.nodes["n1"]?.props).toEqual({ fill: "red", opacity: 0.5 });
  });

  it("redoes an undone command", () => {
    const { doc, bus } = setup();
    bus.invoke("doc.addNode", doc, { type: "layer" });
    bus.undo(doc);
    expect(Object.keys(doc.nodes)).toHaveLength(0);
    expect(bus.canRedo()).toBe(true);
    expect(bus.redo(doc)).toBe(true);
    expect(doc.nodes["n1"]?.type).toBe("layer");
    expect(bus.canRedo()).toBe(false);
  });

  it("clears the redo branch once a new edit lands", () => {
    const { doc, bus } = setup();
    bus.invoke("doc.addNode", doc, { type: "layer" });
    bus.undo(doc);
    expect(bus.canRedo()).toBe(true);
    bus.invoke("doc.addNode", doc, { type: "shape" });
    expect(bus.canRedo()).toBe(false);
  });

  it("returns false when there is nothing to undo or redo", () => {
    const { doc, bus } = setup();
    expect(bus.undo(doc)).toBe(false);
    expect(bus.redo(doc)).toBe(false);
  });
});
