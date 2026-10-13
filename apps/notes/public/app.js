/**
 * Notes — frontend
 * Vanilla ES-module. Talks to the local server via fetch.
 * Same command-bus pattern as PDF Workbench: every mutation goes through /api/op.
 */

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let state = {
  notes: [],
  docVersion: 0,
  canUndo: false,
  canRedo: false,
};

/** The note id currently open in the editor. */
let activeId = null;

/** Whether we're showing the markdown preview. */
let previewMode = false;

/** Pending auto-save timer. */
let saveTimer = null;

// ---------------------------------------------------------------------------
// DOM refs
// ---------------------------------------------------------------------------

const $ = id => document.getElementById(id);

const noteList      = $("note-list");
const searchInput   = $("search");
const newNoteBtn    = $("new-note");
const titleInput    = $("title-input");
const editor        = $("editor");
const preview       = $("preview");
const emptyState    = $("empty-state");
const undoBtn       = $("undo");
const redoBtn       = $("redo");
const toggleBtn     = $("toggle-preview");
const exportBtn     = $("export");
const deleteBtn     = $("delete-note");
const saveProjectBtn = $("save-project");
const projectInput  = $("project-input");
const statusEl      = $("status");
const wordCountEl   = $("word-count");
const noteCountEl   = $("note-count");

// ---------------------------------------------------------------------------
// API helpers — same shape as pdf-workbench
// ---------------------------------------------------------------------------

async function apiFetch(path, options = {}) {
  const res = await fetch(path, options);
  return res.json();
}

async function op(name, params = {}) {
  return apiFetch("/api/op", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ name, params }),
  });
}

// ---------------------------------------------------------------------------
// Lightweight Markdown renderer
// No external deps — handles the most common Markdown constructs.
// ---------------------------------------------------------------------------

function renderMarkdown(src) {
  if (!src) return "";

  // Escape HTML entities first
  const esc = s => s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  // Process fenced code blocks before line-by-line
  src = src.replace(/```(\w*)\n([\s\S]*?)```/g, (_, lang, code) => {
    return `<pre><code class="lang-${esc(lang)}">${esc(code.trimEnd())}</code></pre>`;
  });

  const lines = src.split("\n");
  const out = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i] ?? "";

    // Already-processed HTML (fenced code blocks)
    if (line.startsWith("<pre>")) {
      out.push(line);
      i++;
      continue;
    }

    // Headings
    const hMatch = line.match(/^(#{1,3})\s+(.*)/);
    if (hMatch) {
      const level = hMatch[1].length;
      out.push(`<h${level}>${inlineMarkdown(hMatch[2])}</h${level}>`);
      i++;
      continue;
    }

    // Horizontal rule
    if (/^---+$/.test(line.trim())) {
      out.push("<hr />");
      i++;
      continue;
    }

    // Blockquote
    if (line.startsWith("> ")) {
      out.push(`<blockquote><p>${inlineMarkdown(line.slice(2))}</p></blockquote>`);
      i++;
      continue;
    }

    // Unordered list
    if (/^[-*+]\s/.test(line)) {
      const items = [];
      while (i < lines.length && /^[-*+]\s/.test(lines[i] ?? "")) {
        items.push(`<li>${inlineMarkdown((lines[i] ?? "").slice(2))}</li>`);
        i++;
      }
      out.push(`<ul>${items.join("")}</ul>`);
      continue;
    }

    // Ordered list
    if (/^\d+\.\s/.test(line)) {
      const items = [];
      while (i < lines.length && /^\d+\.\s/.test(lines[i] ?? "")) {
        items.push(`<li>${inlineMarkdown((lines[i] ?? "").replace(/^\d+\.\s/, ""))}</li>`);
        i++;
      }
      out.push(`<ol>${items.join("")}</ol>`);
      continue;
    }

    // Empty line
    if (line.trim() === "") {
      i++;
      continue;
    }

    // Paragraph
    const para = [];
    while (i < lines.length && (lines[i] ?? "").trim() !== "" && !isBlock(lines[i] ?? "")) {
      para.push(lines[i]);
      i++;
    }
    out.push(`<p>${inlineMarkdown(para.join(" "))}</p>`);
  }

  return out.join("\n");
}

function isBlock(line) {
  return (
    /^#{1,3}\s/.test(line) ||
    /^[-*+]\s/.test(line) ||
    /^\d+\.\s/.test(line) ||
    /^> /.test(line) ||
    /^---+$/.test(line.trim()) ||
    line.startsWith("<pre>")
  );
}

function inlineMarkdown(s) {
  // code
  s = s.replace(/`([^`]+)`/g, (_, c) => `<code>${c}</code>`);
  // bold
  s = s.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/__(.+?)__/g, "<strong>$1</strong>");
  // italic
  s = s.replace(/\*(.+?)\*/g, "<em>$1</em>");
  s = s.replace(/_(.+?)_/g, "<em>$1</em>");
  // link
  s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer">$1</a>');
  return s;
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

function relativeDate(iso) {
  if (!iso) return "";
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1)   return "just now";
  if (mins < 60)  return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24)   return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  if (days < 7)   return `${days}d ago`;
  return new Date(iso).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function renderNoteList() {
  const query = searchInput.value.trim().toLowerCase();
  const filtered = query
    ? state.notes.filter(n =>
        n.title.toLowerCase().includes(query) ||
        n.content.toLowerCase().includes(query)
      )
    : state.notes;

  if (filtered.length === 0) {
    noteList.innerHTML = `<li class="note-list-empty">${
      query ? "No notes match your search." : "No notes yet."
    }</li>`;
    return;
  }

  noteList.innerHTML = filtered.map(n => `
    <li class="note-item${n.id === activeId ? " active" : ""}" data-id="${n.id}">
      <div class="note-item-title">${escHtml(n.title || "Untitled")}</div>
      <div class="note-item-snippet">${escHtml(n.snippet || "No content")}</div>
      <div class="note-item-date">${relativeDate(n.updatedAt)}</div>
    </li>
  `).join("");

  noteList.querySelectorAll(".note-item").forEach(el => {
    el.addEventListener("click", () => openNote(el.dataset.id));
  });
}

function renderEditor() {
  const note = state.notes.find(n => n.id === activeId);

  if (!note) {
    titleInput.value = "";
    editor.value = "";
    emptyState.hidden = false;
    editor.hidden = true;
    preview.hidden = true;
    titleInput.disabled = true;
    deleteBtn.hidden = true;
    wordCountEl.textContent = "";
    return;
  }

  titleInput.disabled = false;
  deleteBtn.hidden = false;
  emptyState.hidden = true;

  // Only update if content changed externally (avoid caret jump during typing)
  if (titleInput.value !== note.title) titleInput.value = note.title;
  if (editor.value !== note.content)   editor.value = note.content;

  if (previewMode) {
    editor.hidden = true;
    preview.hidden = false;
    preview.innerHTML = renderMarkdown(note.content);
  } else {
    editor.hidden = false;
    preview.hidden = true;
    editor.focus();
  }

  const words = (note.content ?? "").trim().split(/\s+/).filter(Boolean).length;
  wordCountEl.textContent = `${words} word${words !== 1 ? "s" : ""}`;
}

function renderToolbar() {
  undoBtn.disabled = !state.canUndo;
  redoBtn.disabled = !state.canRedo;
  noteCountEl.textContent = `${state.notes.length} note${state.notes.length !== 1 ? "s" : ""}`;
}

function render() {
  renderNoteList();
  renderEditor();
  renderToolbar();
}

function escHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

function setStatus(msg, duration = 2000) {
  statusEl.textContent = msg;
  if (duration > 0) setTimeout(() => { statusEl.textContent = "Ready"; }, duration);
}

function applyState(data) {
  if (!data.ok) {
    setStatus(`Error: ${data.error?.message ?? "unknown"}`);
    return;
  }
  state = { notes: data.notes, docVersion: data.docVersion, canUndo: data.canUndo, canRedo: data.canRedo };
  render();
}

async function openNote(id) {
  activeId = id;
  previewMode = false;
  toggleBtn.textContent = "Preview";
  render();
}

async function createNote() {
  const data = await op("note.create", { title: "Untitled", content: "" });
  applyState(data);
  if (data.ok && data.result?.id) {
    await openNote(data.result.id);
  }
  setStatus("Note created");
}

function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(flushSave, 600);
}

async function flushSave() {
  if (!activeId) return;
  const data = await op("note.update", {
    id: activeId,
    title: titleInput.value,
    content: editor.value,
  });
  applyState(data);
  setStatus("Saved", 1200);
}

async function deleteNote() {
  if (!activeId) return;
  const note = state.notes.find(n => n.id === activeId);
  if (!confirm(`Delete "${note?.title || "this note"}"? This can be undone.`)) return;
  const data = await op("note.delete", { id: activeId });
  activeId = null;
  applyState(data);
  setStatus("Note deleted");
}

// ---------------------------------------------------------------------------
// Event listeners
// ---------------------------------------------------------------------------

newNoteBtn.addEventListener("click", createNote);

searchInput.addEventListener("input", renderNoteList);

titleInput.addEventListener("input", scheduleSave);
editor.addEventListener("input", () => {
  // Update word count immediately
  const words = editor.value.trim().split(/\s+/).filter(Boolean).length;
  wordCountEl.textContent = `${words} word${words !== 1 ? "s" : ""}`;
  scheduleSave();
});

undoBtn.addEventListener("click", async () => {
  const data = await apiFetch("/api/undo", { method: "POST" });
  // After undo, re-sync editor with server state
  applyState(data);
  // If the active note was deleted by undo, pick next
  if (activeId && !state.notes.find(n => n.id === activeId)) {
    activeId = state.notes[0]?.id ?? null;
  }
  render();
});

redoBtn.addEventListener("click", async () => {
  const data = await apiFetch("/api/redo", { method: "POST" });
  applyState(data);
  render();
});

toggleBtn.addEventListener("click", () => {
  if (!activeId) return;
  // Flush any pending save before switching to preview
  clearTimeout(saveTimer);
  if (!previewMode) flushSave();
  previewMode = !previewMode;
  toggleBtn.textContent = previewMode ? "Edit" : "Preview";
  renderEditor();
});

exportBtn.addEventListener("click", () => {
  window.location.href = "/api/export";
  setStatus("Exporting…");
});

deleteBtn.addEventListener("click", deleteNote);

saveProjectBtn.addEventListener("click", () => {
  window.location.href = "/api/project";
  setStatus("Saving project…");
});

projectInput.addEventListener("change", async () => {
  const file = projectInput.files[0];
  if (!file) return;
  const text = await file.text();
  const data = await apiFetch("/api/project", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: text,
  });
  activeId = null;
  applyState(data);
  setStatus("Project loaded");
  projectInput.value = "";
});

// Keyboard shortcuts
document.addEventListener("keydown", e => {
  const ctrl = e.ctrlKey || e.metaKey;
  if (ctrl && e.key === "n") { e.preventDefault(); createNote(); }
  if (ctrl && e.key === "z" && !e.shiftKey) { e.preventDefault(); undoBtn.click(); }
  if (ctrl && (e.key === "Z" || (e.key === "z" && e.shiftKey))) { e.preventDefault(); redoBtn.click(); }
  if (ctrl && e.key === "p") { e.preventDefault(); toggleBtn.click(); }
  if (ctrl && e.key === "s") { e.preventDefault(); clearTimeout(saveTimer); flushSave(); }
});

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

(async () => {
  const data = await apiFetch("/api/state");
  applyState(data);
  // Open first note if any exist
  if (state.notes.length > 0) {
    await openNote(state.notes[0].id);
  }
})();
