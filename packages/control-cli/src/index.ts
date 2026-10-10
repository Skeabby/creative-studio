#!/usr/bin/env node
/**
 * Frontend #1 of the control layer: a thin CLI over ControlServer.
 *
 *   nucleus command.list
 *   nucleus command.describe '{"name":"doc.addNode"}'
 *   nucleus command.invoke '{"name":"doc.addNode","params":{"type":"layer"}}'
 */
import { CommandBus, createDocument, registerCoreCommands } from "@nucleus/core";
import { ControlServer } from "@nucleus/control";

const [method = "command.list", paramsJson = "{}"] = process.argv.slice(2);

const bus = new CommandBus({ kind: "human", id: "control-cli" });
const state = createDocument("nucleus.cli");
registerCoreCommands(bus);
const server = new ControlServer(bus, state);

const params = paramsJson.trim() ? JSON.parse(paramsJson) : {};
const response = server.handle({ method, params });

process.stdout.write(`${JSON.stringify(response, null, 2)}\n`);
process.exit(response.ok ? 0 : 1);
