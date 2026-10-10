// Capture a pixel-perfect screenshot of a running Nucleus app page via the
// WebView2 / Chromium DevTools Protocol (CDP).
//
// Why not a screen capture: grabbing the desktop at the window's coordinates
// captures whatever is on screen (other windows, the desktop, taskbar) and is
// fragile. Asking the page's own renderer for a screenshot is deterministic and
// never picks up the wrong window.
//
// Usage:
//   1) launch an app (or the launcher) with WebView2 remote debugging enabled, e.g.
//        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9223"
//        assets\nucleus-shell.exe http://127.0.0.1:4300/ --title Nucleus ...
//   2) node tools/screenshot.mjs <debug-url> <out.png> [width] [height] [dsf]
//
// Node >= 22 (global fetch + WebSocket).

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const debugUrl = (process.argv[2] ?? "http://127.0.0.1:9223").replace(/\/$/, "");
const out = process.argv[3] ?? "docs/screenshot-launcher.png";
const width = Number(process.argv[4] ?? 1440);
const height = Number(process.argv[5] ?? 900);
const scale = Number(process.argv[6] ?? 2);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function findPage() {
  const deadline = Date.now() + 20000;
  let lastErr = "no targets";
  while (Date.now() < deadline) {
    try {
      const res = await fetch(debugUrl + "/json");
      const targets = await res.json();
      const page = targets.find((t) => t.type === "page" && t.webSocketDebuggerUrl);
      if (page) return page;
      lastErr = JSON.stringify(targets.map((t) => t.type));
    } catch (err) {
      lastErr = err.message;
    }
    await sleep(500);
  }
  throw new Error(`no debuggable page at ${debugUrl} (last: ${lastErr})`);
}

async function main() {
  const page = await findPage();
  console.log(`page: ${page.title} <${page.url}>`);

  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });

  let id = 0;
  const pending = new Map();
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      pending.get(msg.id)(msg);
      pending.delete(msg.id);
    }
  };
  const send = (method, params = {}) =>
    new Promise((resolve) => {
      const mid = ++id;
      pending.set(mid, resolve);
      ws.send(JSON.stringify({ id: mid, method, params }));
    });

  await send("Page.enable");
  await send("Runtime.enable");
  await send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: scale,
    mobile: false,
  });
  // Give the SPA a moment to finish its first fetch + paint.
  await sleep(1500);

  // Self-check: prove the captured page actually rendered content.
  const probe = await send("Runtime.evaluate", {
    returnByValue: true,
    expression: `JSON.stringify({
      title: document.title,
      cards: document.querySelectorAll('.card, .app-card').length,
      rows: document.querySelectorAll('tbody tr').length,
      groups: document.querySelectorAll('.group').length,
      largeTiles: document.querySelectorAll('.tile-lg').length,
      palette: !!document.querySelector('#palette'),
      drawer: !!document.querySelector('#drawer'),
      textLen: (document.body.innerText || '').length
    })`,
  });
  const stats = probe.result?.result?.value;
  console.log("dom:", stats);
  try {
    const parsed = JSON.parse(stats ?? "{}");
    if (!parsed.cards && !parsed.rows && !parsed.largeTiles) {
      console.warn("WARNING: page rendered no app cards — screenshot may look empty");
    }
  } catch {
    // ignore probe parse issues
  }

  const res = await send("Page.captureScreenshot", {
    format: "png",
    captureBeyondViewport: false,
    optimizeForSpeed: false,
  });

  const data = res.result?.data;
  if (!data) throw new Error("captureScreenshot returned no data: " + JSON.stringify(res).slice(0, 400));

  const buf = Buffer.from(data, "base64");
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, buf);
  console.log(`Captured ${width}x${height} @${scale}x -> ${out} (${buf.length} bytes)`);
  ws.close();
}

main().catch((err) => {
  console.error("screenshot failed:", err.message);
  process.exit(1);
});
