// Minimal schema validation + codegen placeholder.
// Phase 1 will generate TS types, CLI help and MCP tool definitions from here.
import { readdir, readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const schemaDir = join(root, "schemas");

const files = (await readdir(schemaDir)).filter((f) => f.endsWith(".schema.json")).sort();
if (files.length === 0) throw new Error("codegen: no schemas found");

for (const file of files) {
  const schema = JSON.parse(await readFile(join(schemaDir, file), "utf8"));
  if (!schema.$id) throw new Error(`codegen: ${file} is missing $id`);
  if (!schema.$schema) throw new Error(`codegen: ${file} is missing $schema`);
  console.log(`  ok  ${schema.$id}`);
}

console.log(`codegen: validated ${files.length} schema(s)`);
