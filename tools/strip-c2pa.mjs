#!/usr/bin/env node
/**
 * strip-c2pa.mjs — remove C2PA / provenance metadata blobs from asset files.
 *
 * C2PA (and the vendor metadata Claude/etc. embed) can make up 80-90% of an
 * SVG/PNG and ships non-standard namespaces into the product. This tool drops:
 *   - SVG:   any <metadata>...</metadata> block + the xmlns:c2pa attribute
 *   - PNG:   ancillary text/metadata chunks (tEXt/iTXt/zTXt/eXIf) — the image
 *            data itself (IHDR/PLTE/IDAT/IEND) is never touched, so rendering
 *            is bit-identical.
 *
 * Usage:
 *   node tools/strip-c2pa.mjs <file-or-dir> [more paths...]
 *
 * Files are rewritten in place; directories are walked recursively (.svg/.png).
 * Exit code is 0 only if every input SVG/PNG was rewritten or left intact.
 */
import { readFile, writeFile, readdir, stat } from "node:fs/promises";
import { join } from "node:path";
import { crc32 } from "node:zlib";

const SVG_META_RE = /<metadata>[\s\S]*?<\/metadata>/g;
const SVG_NS_RE = /\s+xmlns:c2pa="http:\/\/c2pa\.org\/manifest"/g;

/** PNG ancillary metadata chunks we never need at runtime or in a repo.
 *  `caBX` is the C2PA/Content-Authenticity container (JUMBF) used by most
 *  AI image tools; the rest are plain text/EXIF bags. */
const ANCILLARY = new Set(["tEXt", "iTXt", "zTXt", "eXIf", "caBX"]);

function stripSvg(text) {
  let out = text.replace(SVG_META_RE, "").replace(SVG_NS_RE, "");
  // Tidy the leftover double space between xmlns and viewBox, if any.
  out = out.replace(/<svg(\s)xmlns=([^ >]+)(\s)+xmlns=/, '<svg$1xmlns=$2 ');
  return out;
}

async function stripPng(file) {
  const buf = await readFile(file);
  if (buf.length < 8 || buf.readUInt32BE(0) !== 0x89504e47) {
    throw new Error(`${file}: not a PNG`);
  }
  const parts = [buf.subarray(0, 8)];
  let off = 8;
  let stripped = 0;
  while (off < buf.length) {
    if (off + 8 > buf.length) throw new Error(`${file}: truncated PNG`);
    const len = buf.readUInt32BE(off);
    const type = buf.toString("ascii", off + 4, off + 8);
    const start = off + 8;
    const end = start + len;
    if (end + 4 > buf.length) throw new Error(`${file}: corrupt chunk ${type}`);
    if (ANCILLARY.has(type)) {
      stripped += 1;
      off = end + 4;
      continue;
    }
    parts.push(buf.subarray(off, end + 4));
    off = end + 4;
  }
  // Ancillary chunks are dropped; every kept chunk is byte-identical, so its
  // original CRC stays valid. Verify that on the rebuilt stream.
  const out = Buffer.concat(parts);
  if (stripped === 0) return null;
  let o = 8;
  while (o < out.length) {
    const len = out.readUInt32BE(o);
    const type = out.toString("ascii", o + 4, o + 8);
    const data = out.subarray(o + 8, o + 8 + len);
    const crc = (crc32(data, crc32(Buffer.from(type))) >>> 0);
    if (crc !== out.readUInt32BE(o + 8 + len)) {
      throw new Error(`${file}: CRC mismatch after strip (${type})`);
    }
    o += 12 + len;
  }
  return out;
}

async function processFile(file) {
  const lower = file.toLowerCase();
  if (lower.endsWith(".svg")) {
    const text = await readFile(file, "utf8");
    const out = stripSvg(text);
    if (out !== text) {
      await writeFile(file, out, "utf8");
      return `stripped  ${file}`;
    }
    return `intact    ${file}`;
  }
  if (lower.endsWith(".png")) {
    const out = await stripPng(file);
    if (out) {
      await writeFile(file, out);
      return `stripped  ${file}`;
    }
    return `intact    ${file}`;
  }
  return `ignored   ${file}`;
}

async function walk(path, report) {
  const info = await stat(path);
  if (info.isFile()) {
    report.push(await processFile(path));
    return;
  }
  if (!info.isDirectory()) return;
  for (const entry of await readdir(path)) {
    await walk(join(path, entry), report);
  }
}

const targets = process.argv.slice(2);
if (targets.length === 0) {
  console.error("usage: node tools/strip-c2pa.mjs <file-or-dir> [...]");
  process.exit(2);
}

let failed = false;
for (const target of targets) {
  const report = [];
  try {
    await walk(target, report);
    for (const line of report) console.log(line);
  } catch (error) {
    failed = true;
    console.error(`error: ${error instanceof Error ? error.message : String(error)}`);
  }
}
process.exit(failed ? 1 : 0);