#!/usr/bin/env node
/**
 * Frontend #1 of the control layer: a thin CLI over ControlServer.
 *
 *   studio command.list
 *   studio command.describe '{"name":"doc.addNode"}'
 *   studio command.invoke '{"name":"doc.addNode","params":{"type":"layer"}}'
 */
import { CommandBus, createDocument, registerCoreCommands } from "@studio/core";
import { ControlServer } from "@studio/control";

const [method = "command.list", paramsJson = "{}"] = process.argv.slice(2);

const bus = new CommandBus({ kind: "human", id: "control-cli" });
const state = createDocument("studio.cli");
registerCoreCommands(bus);
const server = new ControlServer(bus, state);

const params = paramsJson.trim() ? JSON.parse(paramsJson) : {};
const response = server.handle({ method, params });

process.stdout.write(`${JSON.stringify(response, null, 2)}\n`);
process.exit(response.ok ? 0 : 1);
