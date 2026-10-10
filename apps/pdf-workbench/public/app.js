// PDF Workbench front-end. Talks to the local server's /api control surface.

const $ = (sel) => document.querySelector(sel);
const grid = $("#grid");
const empty = $("#empty");
const statusEl = $("#status");
const counterEl = $("#counter");
const undoBtn = $("#undo");
const redoBtn = $("#redo");

/** Friendly display names per sourceId (client-side only). */
const sourceNames = new Map();
let state = { pages: [], canUndo: false, canRedo: false };
let busy = false;

// --- pdf.js (optional: thumbnails degrade gracefully if unavailable) -------
let pdfjsLib = null;
try {
  pdfjsLib = await import("/vendor/pdfjs/pdf.mjs");
  pdfjsLib.GlobalWorkerOptions.workerSrc = "/vendor/pdfjs/pdf.worker.mjs";
} catch (error) {
  console.warn("[workbench] pdf.js unavailable — page numbers only", error);
}

const THUMB_WIDTH = 160;
const thumbCache = new Map(); // "sourceId:sourcePage" -> dataURL
const sourceDocs = new Map(); // "sourceId" -> Promise<PDFDocumentProxy>

function getSource(sourceId) {
  let doc = sourceDocs.get(sourceId);
  if (!doc) {
    doc = pdfjsLib.getDocument({ url: `/api/source/${encodeURIComponent(sourceId)}` }).promise;
    sourceDocs.set(sourceId, doc);
  }
  return doc;
}

async function renderThumb(sourceId, sourcePage) {
  const key = `${sourceId}:${sourcePage}`;
  const cached = thumbCache.get(key);
  if (cached) return cached;
  const pdf = await getSource(sourceId);
  const page = await pdf.getPage(sourcePage + 1);
  const base = page.getViewport({ scale: 1 });
  const viewport = page.getViewport({ scale: THUMB_WIDTH / base.width });
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const canvas = document.createElement("canvas");
  canvas.width = Math.floor(viewport.width * dpr);
  canvas.height = Math.floor(viewport.height * dpr);
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, viewport.width, viewport.height);
  await page.render({ canvasContext: ctx, canvas, viewport }).promise;
  const url = canvas.toDataURL("image/jpeg", 0.72);
  thumbCache.set(key, url);
  return url;
}

const pendingThumbs = new Map(); // element -> { sourceId, sourcePage, img }
const observer = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      const info = pendingThumbs.get(entry.target);
      observer.unobserve(entry.target);
      pendingThumbs.delete(entry.target);
      if (!info || !pdfjsLib) continue;
      renderThumb(info.sourceId, info.sourcePage)
        .then((url) => { if (url) { info.img.src = url; } })
        .catch(() => {});
    }
  },
  { rootMargin: "400px" },
);

// --- server plumbing -------------------------------------------------------
const setStatus = (msg) => { statusEl.textContent = msg; };

async function post(url, body) {
  const res = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body ?? {}),
  });
  return res.json();
}

function apply(payload) {
  if (!payload.ok) { setStatus(`Error: ${payload.error?.message ?? "unknown"}`); return; }
  state.pages = payload.pages ?? [];
  state.canUndo = Boolean(payload.canUndo);
  state.canRedo = Boolean(payload.canRedo);
  render();
}

async function refresh() {
  apply(await (await fetch("/api/state")).json());
}

function fileToBase64(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).split(",")[1]);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

async function loadFiles(fileList) {
  const files = [...fileList].filter((f) => f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf"));
  if (files.length === 0) return;
  let done = 0;
  for (const file of files) {
    setStatus(`Loading ${file.name} (${++done}/${files.length})…`);
    const dataBase64 = await fileToBase64(file);
    const loaded = await post("/api/load", { dataBase64, name: file.name });
    if (!loaded.ok) { setStatus(`Failed: ${loaded.error?.message ?? "unknown"}`); continue; }
    sourceNames.set(loaded.sourceId, file.name);
    const pages = Array.from({ length: loaded.pageCount }, (_, i) => i);
    apply(await post("/api/op", { name: "pdf.importPages", params: { sourceId: loaded.sourceId, pages } }));
  }
  setStatus(`Added ${files.length} file${files.length > 1 ? "s" : ""}.`);
}

// --- rendering -------------------------------------------------------------
function rotationTransform(rotation) {
  return rotation % 180 === 0 ? `rotate(${rotation}deg)` : `rotate(${rotation}deg) scale(0.72)`;
}

function render() {
  const pages = state.pages;
  empty.style.display = pages.length === 0 ? "block" : "none";
  undoBtn.disabled = !state.canUndo || busy;
  redoBtn.disabled = !state.canRedo || busy;
  counterEl.textContent = pages.length ? `${pages.length} page${pages.length > 1 ? "s" : ""}` : "";

  const frag = document.createDocumentFragment();

  pages.forEach((page, index) => {
    const card = document.createElement("div");
    card.className = "card";
    card.draggable = true;
    card.dataset.index = String(index);
    card.dataset.id = page.id;

    const thumb = document.createElement("div");
    thumb.className = "thumb";
    const img = new Image();
    img.className = "thumb-img";
    img.alt = "";
    img.style.transform = rotationTransform(page.rotation);
    const no = document.createElement("span");
    no.className = "page-no";
    no.textContent = String(page.sourcePage + 1);
    const badge = document.createElement("span");
    badge.className = "rot-badge";
    badge.dataset.deg = String(page.rotation);
    badge.textContent = `${page.rotation}°`;
    thumb.append(img, no, badge);

    const key = `${page.sourceId}:${page.sourcePage}`;
    if (thumbCache.has(key)) {
      img.src = thumbCache.get(key);
    } else if (pdfjsLib) {
      pendingThumbs.set(thumb, { sourceId: page.sourceId, sourcePage: page.sourcePage, img });
      observer.observe(thumb);
    }

    const meta = document.createElement("div");
    meta.className = "meta";
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = sourceNames.get(page.sourceId) ?? page.sourceId.slice(0, 8);
    const pos = document.createElement("span");
    pos.textContent = `#${index + 1}`;
    meta.append(name, pos);

    const tools = document.createElement("div");
    tools.className = "tools";
    const mk = (label, title, fn, danger = false) => {
      const b = document.createElement("button");
      b.textContent = label;
      b.title = title;
      if (danger) b.className = "danger";
      b.onclick = (e) => { e.stopPropagation(); fn(); };
      return b;
    };
    tools.append(
      mk("↺", "Rotate left 90°", () => op("pdf.rotatePage", { id: page.id, degrees: -90 })),
      mk("↻", "Rotate right 90°", () => op("pdf.rotatePage", { id: page.id, degrees: 90 })),
      mk("←", "Move earlier", () => op("pdf.movePage", { id: page.id, to: index - 1 })),
      mk("→", "Move later", () => op("pdf.movePage", { id: page.id, to: index + 1 })),
      mk("✕", "Remove page", () => op("pdf.removePage", { id: page.id }), true),
    );

    card.append(thumb, meta, tools);
    frag.appendChild(card);
  });

  grid.replaceChildren(frag);
}

async function op(name, params) {
  if (busy) return;
  busy = true;
  setStatus(`${name}…`);
  try {
    apply(await post("/api/op", { name, params }));
    setStatus("Ready");
  } finally {
    busy = false;
    undoBtn.disabled = !state.canUndo;
    redoBtn.disabled = !state.canRedo;
  }
}

async function undo() {
  apply(await post("/api/undo"));
  setStatus(state.canUndo ? "Undo" : "Nothing left to undo");
}

async function redo() {
  apply(await post("/api/redo"));
  setStatus(state.canRedo ? "Redo" : "Nothing left to redo");
}

async function doExport() {
  if (state.pages.length === 0) { setStatus("Nothing to export — add a PDF first."); return; }
  setStatus("Building PDF…");
  const res = await fetch("/api/export");
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    setStatus(`Export failed: ${err.error?.message ?? res.statusText}`);
    return;
  }
  const blob = await res.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "workbench.pdf";
  a.click();
  URL.revokeObjectURL(url);
  setStatus(`Exported ${state.pages.length} page(s).`);
}

// --- drag to reorder -------------------------------------------------------
let dragIndex = null;

grid.addEventListener("dragstart", (e) => {
  const card = e.target.closest(".card");
  if (!card) return;
  dragIndex = Number(card.dataset.index);
  card.classList.add("dragging");
  e.dataTransfer.effectAllowed = "move";
});
grid.addEventListener("dragend", (e) => {
  e.target.closest(".card")?.classList.remove("dragging");
  dragIndex = null;
});
grid.addEventListener("dragover", (e) => {
  e.preventDefault();
  e.dataTransfer.dropEffect = "move";
});
grid.addEventListener("drop", (e) => {
  const card = e.target.closest(".card");
  if (!card || dragIndex === null) return;
  e.preventDefault();
  const to = Number(card.dataset.index);
  const id = grid.children[dragIndex]?.dataset.id;
  if (id && to !== dragIndex) op("pdf.movePage", { id, to });
  dragIndex = null;
});

// --- wiring ----------------------------------------------------------------
$("#file-input").addEventListener("change", (e) => loadFiles(e.target.files));
$("#export").addEventListener("click", doExport);
undoBtn.addEventListener("click", undo);
redoBtn.addEventListener("click", redo);
$("#reset").addEventListener("click", async () => {
  sourceNames.clear();
  thumbCache.clear();
  sourceDocs.clear();
  apply(await post("/api/reset"));
  setStatus("Reset.");
});

$("#save").addEventListener("click", () => {
  if (state.pages.length === 0) { setStatus("Nothing to save yet."); return; }
  setStatus("Saving project…");
  const a = document.createElement("a");
  a.href = "/api/project";
  a.download = "workbench.studio.json";
  a.click();
});

$("#project-input").addEventListener("change", async (e) => {
  const file = e.target.files?.[0];
  if (!file) return;
  setStatus(`Opening ${file.name}…`);
  try {
    const text = await file.text();
    const payload = JSON.parse(text);
    sourceNames.clear();
    thumbCache.clear();
    sourceDocs.clear();
    apply(await post("/api/project", payload));
    setStatus(`Opened ${file.name}.`);
  } catch (error) {
    setStatus(`Open failed: ${error instanceof Error ? error.message : String(error)}`);
  }
  e.target.value = "";
});

document.addEventListener("keydown", (e) => {
  const mod = e.ctrlKey || e.metaKey;
  if (!mod) return;
  const key = e.key.toLowerCase();
  if (key === "z" && !e.shiftKey) { e.preventDefault(); void undo(); }
  else if ((key === "z" && e.shiftKey) || key === "y") { e.preventDefault(); void redo(); }
});

const dropzone = $("#dropzone");
dropzone.addEventListener("dragover", (e) => { e.preventDefault(); dropzone.classList.add("dragover"); });
dropzone.addEventListener("dragleave", () => dropzone.classList.remove("dragover"));
dropzone.addEventListener("drop", (e) => {
  e.preventDefault();
  dropzone.classList.remove("dragover");
  if (e.dataTransfer?.files?.length) void loadFiles(e.dataTransfer.files);
});

refresh();
