// Nucleus launcher UI.

const $ = (sel) => document.querySelector(sel);

const gridEl = $("#grid");
const tableWrap = $("#table-wrap");
const tableBody = $("#table-body");
const emptyEl = $("#empty");
const emptyText = $("#empty-text");
const statusEl = $("#status");
const countEl = $("#count");
const runningCountEl = $("#running-count");
const plannedCountEl = $("#planned-count");
const searchEl = $("#search");
const root = document.documentElement;

const paletteEl = $("#palette");
const paletteInput = $("#palette-input");
const paletteList = $("#palette-list");
const drawerEl = $("#drawer");
const drawerPanel = $("#drawer-panel");

let apps = [];
let view = localStorage.getItem("nucleus.view") ?? "grid";
let query = "";
let sort = parseSort();
const pending = new Set();
let timer = null;
let loaded = false;

let order = [];            // visible apps in render order (keyboard nav)
let selectedId = null;     // keyboard selection
let detailId = null;       // open drawer
let paletteActive = false;
let paletteItems = [];
let paletteIndex = 0;
let paletteQuery = "";

const setStatus = (msg) => { statusEl.textContent = msg; };

// --- theme -----------------------------------------------------------------
function applyTheme(theme) {
  root.dataset.theme = theme;
  $("#theme").textContent = theme === "dark" ? "☾" : "☀";
  localStorage.setItem("nucleus.theme", theme);
}
function toggleTheme() {
  applyTheme(root.dataset.theme === "dark" ? "light" : "dark");
}
applyTheme(localStorage.getItem("nucleus.theme") ?? "dark");
$("#theme").addEventListener("click", toggleTheme);

// --- view toggle -----------------------------------------------------------
function applyView(next) {
  view = next;
  localStorage.setItem("nucleus.view", next);
  for (const button of document.querySelectorAll(".segmented button")) {
    button.classList.toggle("active", button.dataset.view === next);
  }
  gridEl.hidden = next !== "grid";
  tableWrap.hidden = next !== "table";
  render();
}
for (const button of document.querySelectorAll(".segmented button")) {
  button.addEventListener("click", () => applyView(button.dataset.view));
}

// --- table sorting ---------------------------------------------------------
function parseSort() {
  try {
    const raw = JSON.parse(localStorage.getItem("nucleus.sort") ?? "");
    if (raw && typeof raw.key === "string" && (raw.dir === "asc" || raw.dir === "desc")) return raw;
  } catch {
    /* ignore */
  }
  return { key: "name", dir: "asc" };
}
function setSort(key) {
  if (sort.key === key) sort.dir = sort.dir === "asc" ? "desc" : "asc";
  else { sort.key = key; sort.dir = "asc"; }
  localStorage.setItem("nucleus.sort", JSON.stringify(sort));
}
for (const th of document.querySelectorAll("th[data-sort]")) {
  th.addEventListener("click", () => { setSort(th.dataset.sort); render(); });
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });
function sortValue(app, key) {
  switch (key) {
    case "inspiredBy": return app.inspiredBy ?? "";
    case "category": return groupTitle(app.category);
    case "status": return app.status;
    case "version": return app.version;
    case "source": return app.source;
    default: return app.name;
  }
}
function sortTable(list) {
  const dir = sort.dir === "asc" ? 1 : -1;
  return list.slice().sort((a, b) => {
    const ap = a.status === "planned" ? 1 : 0;
    const bp = b.status === "planned" ? 1 : 0;
    if (ap !== bp) return ap - bp;
    return dir * collator.compare(String(sortValue(a, sort.key)), String(sortValue(b, sort.key)));
  });
}

// --- category grouping -----------------------------------------------------
const GROUP_DEFS = [
  { title: "Image & Design", cats: ["image"] },
  { title: "Documents", cats: ["pdf", "text", "layout"] },
  { title: "Data & Engineering", cats: ["data", "cad"] },
  { title: "System", cats: ["system"] },
  { title: "Other", cats: ["other"] },
];
const GROUP_ORDER = GROUP_DEFS.map((g) => g.title);
function groupTitle(category) {
  for (const group of GROUP_DEFS) if (group.cats.includes(category)) return group.title;
  return "Other";
}
function rank(app) {
  if (app.status === "planned") return 3;
  if (app.self) return 2;
  if (app.running) return 0;
  return 1;
}
function byRank(a, b) {
  if (rank(a) !== rank(b)) return rank(a) - rank(b);
  return collator.compare(a.name, b.name);
}
function groupOf(list) {
  const buckets = new Map();
  for (const app of list) {
    const title = groupTitle(app.category);
    if (!buckets.has(title)) buckets.set(title, []);
    buckets.get(title).push(app);
  }
  return GROUP_ORDER.filter((title) => buckets.has(title)).map((title) => ({
    title,
    items: buckets.get(title).slice().sort(byRank),
  }));
}

// --- data ------------------------------------------------------------------
async function loadApps() {
  try {
    const response = await fetch("/api/apps");
    const payload = await response.json();
    if (!payload.ok) throw new Error(payload.error?.message ?? "failed");
    apps = payload.apps ?? [];
    loaded = true;
    render();
  } catch (error) {
    loaded = true;
    setStatus(`Could not reach the launcher: ${error.message}`);
  }
}

function visibleApps() {
  const needle = query.trim().toLowerCase();
  if (!needle) return apps;
  return apps.filter((app) =>
    [app.name, app.id, app.description, app.category, app.inspiredBy, app.status, ...(app.capabilities ?? [])]
      .filter(Boolean)
      .join(" ")
      .toLowerCase()
      .includes(needle));
}

function refreshFrom(payload) {
  if (payload.apps) apps = payload.apps;
}

async function openApp(app) {
  pending.add(app.id);
  setStatus(`Starting ${app.name}…`);
  render();
  try {
    const response = await fetch(`/api/apps/${encodeURIComponent(app.id)}/open`, { method: "POST" });
    const payload = await response.json();
    refreshFrom(payload);
    setStatus(payload.ok ? `${app.name} is opening in its own window.` : `Failed: ${payload.error?.message}`);
  } catch (error) {
    setStatus(`Failed to start ${app.name}: ${error.message}`);
  } finally {
    pending.delete(app.id);
    render();
  }
}

async function stopApp(app) {
  pending.add(app.id);
  setStatus(`Stopping ${app.name}…`);
  render();
  try {
    const response = await fetch(`/api/apps/${encodeURIComponent(app.id)}/stop`, { method: "POST" });
    const payload = await response.json();
    refreshFrom(payload);
    setStatus(`${app.name} stopped.`);
  } finally {
    pending.delete(app.id);
    render();
  }
}

function primaryAction(app) {
  if (app.self || app.status === "planned" || !app.launchable || app.running) openDetail(app.id);
  else openApp(app);
}

// --- helpers ---------------------------------------------------------------
function statusPill(status, floating = true) {
  const span = document.createElement("span");
  span.className = `pill pill-${status}${floating ? "" : " pill-inline"}`;
  span.textContent = status;
  return span;
}

function iconTile(app, size, withDot = false) {
  const tile = document.createElement("div");
  tile.className = `tile tile-${size}${withDot && app.running && !app.self ? " has-dot" : ""}`;
  if (app.iconUrl) {
    const img = document.createElement("img");
    img.src = app.iconUrl;
    img.alt = "";
    tile.appendChild(img);
  } else {
    tile.style.background = `linear-gradient(145deg, ${app.color ?? "#7c6bff"}, color-mix(in srgb, ${app.color ?? "#7c6bff"} 55%, #10131c))`;
    tile.textContent = app.emoji ?? "✨";
  }
  return tile;
}

function actionButton(app) {
  const busy = pending.has(app.id);
  const button = document.createElement("button");
  button.className = "btn";

  if (app.self) {
    button.textContent = "This window";
    button.disabled = true;
    return button;
  }
  if (app.status === "planned" || !app.launchable) {
    button.textContent = "Coming soon";
    button.disabled = true;
    return button;
  }
  if (busy) {
    button.textContent = "Starting…";
    button.disabled = true;
    return button;
  }
  if (app.running) {
    button.textContent = "Open";
    button.addEventListener("click", () => openApp(app));
    const stop = document.createElement("button");
    stop.className = "btn subtle";
    stop.textContent = "Stop";
    stop.title = "Stop this app";
    stop.addEventListener("click", () => stopApp(app));
    const wrap = document.createElement("div");
    wrap.className = "btn-row";
    wrap.append(button, stop);
    return wrap;
  }
  button.textContent = "Open";
  button.className = "btn primary";
  button.addEventListener("click", () => openApp(app));
  return button;
}

// --- rendering -------------------------------------------------------------
function cardFor(app) {
  const card = document.createElement("article");
  card.className = "card";
  card.dataset.id = app.id;
  card.tabIndex = 0;
  if (app.status === "planned") card.classList.add("planned");
  if (app.running && !app.self) card.classList.add("running");

  card.addEventListener("click", (event) => {
    if (event.target.closest("button")) return;
    selectedId = app.id;
    applySelection();
    openDetail(app.id);
  });
  card.addEventListener("focus", () => { selectedId = app.id; applySelection(); });

  const head = document.createElement("div");
  head.className = "card-head";
  head.append(iconTile(app, "lg", true));
  const title = document.createElement("div");
  title.className = "card-title";
  const name = document.createElement("strong");
  name.textContent = app.name;
  const meta = document.createElement("span");
  meta.className = "card-meta";
  meta.textContent = app.version === "—" ? groupTitle(app.category) : `v${app.version} · ${groupTitle(app.category)}`;
  title.append(name, meta);
  head.append(title);

  const description = document.createElement("p");
  description.className = "card-desc";
  description.textContent = app.description || "No description yet.";

  const foot = document.createElement("div");
  foot.className = "card-foot";
  const inspired = document.createElement("span");
  inspired.className = "inspired";
  inspired.textContent = app.inspiredBy ? `like ${app.inspiredBy}` : "";
  foot.append(inspired, actionButton(app));

  card.append(statusPill(app.status), head, description, foot);
  return card;
}

function renderGrid(list) {
  const frag = document.createDocumentFragment();
  for (const group of groupOf(list)) {
    const section = document.createElement("section");
    section.className = "group";

    const head = document.createElement("header");
    head.className = "group-head";
    const h2 = document.createElement("h2");
    h2.textContent = group.title;
    const count = document.createElement("span");
    count.className = "group-count";
    count.textContent = `${group.items.length}`;
    head.append(h2, count);

    const cards = document.createElement("div");
    cards.className = "cards";
    for (const app of group.items) cards.append(cardFor(app));

    section.append(head, cards);
    frag.append(section);
  }
  gridEl.replaceChildren(frag);
}

function rowFor(app) {
  const row = document.createElement("tr");
  row.dataset.id = app.id;
  row.tabIndex = 0;
  if (app.status === "planned") row.classList.add("planned");
  row.addEventListener("click", (event) => {
    if (event.target.closest("button")) return;
    selectedId = app.id;
    applySelection();
    openDetail(app.id);
  });
  row.addEventListener("focus", () => { selectedId = app.id; applySelection(); });

  const appCell = document.createElement("td");
  const appWrap = document.createElement("div");
  appWrap.className = "row-app";
  appWrap.append(iconTile(app, "sm", true));
  const nameWrap = document.createElement("div");
  const strong = document.createElement("strong");
  strong.textContent = app.name;
  const small = document.createElement("span");
  small.className = "muted";
  small.textContent = app.description;
  nameWrap.append(strong, small);
  appWrap.append(nameWrap);
  appCell.append(appWrap);

  const inspired = document.createElement("td");
  inspired.className = app.inspiredBy ? "" : "muted";
  inspired.textContent = app.inspiredBy || "—";
  const category = document.createElement("td");
  category.textContent = groupTitle(app.category);

  const statusCell = document.createElement("td");
  statusCell.append(statusPill(app.status, false));

  const version = document.createElement("td");
  version.textContent = app.version;

  const source = document.createElement("td");
  source.className = "muted";
  source.textContent = app.source;

  const actionCell = document.createElement("td");
  actionCell.className = "right";
  actionCell.append(actionButton(app));

  row.append(appCell, inspired, category, statusCell, version, source, actionCell);
  return row;
}

function renderTable(list) {
  const frag = document.createDocumentFragment();
  for (const app of sortTable(list)) frag.append(rowFor(app));
  tableBody.replaceChildren(frag);
}

function renderSortArrows() {
  for (const th of document.querySelectorAll("th[data-sort]")) {
    th.querySelector(".arrow")?.remove();
    if (th.dataset.sort !== sort.key) continue;
    const span = document.createElement("span");
    span.className = "arrow";
    span.textContent = sort.dir === "asc" ? "▲" : "▼";
    th.append(span);
  }
}

function applySelection() {
  for (const el of document.querySelectorAll("[data-id]")) {
    el.classList.toggle("selected", selectedId !== null && el.dataset.id === selectedId);
  }
}

function render() {
  const list = visibleApps();
  order = view === "grid" ? groupOf(list).flatMap((g) => g.items) : sortTable(list);
  if (selectedId && !order.some((app) => app.id === selectedId)) selectedId = null;

  const ready = apps.filter((a) => a.status !== "planned").length;
  const planned = apps.filter((a) => a.status === "planned").length;
  const running = apps.filter((a) => a.running && !a.self).length;

  countEl.textContent = loaded ? `${ready} apps · ${planned} planned` : "loading…";
  runningCountEl.textContent = running > 0 ? `● ${running} running` : "";
  plannedCountEl.textContent = planned > 0 ? `${planned} planned` : "";

  if (view === "grid") renderGrid(list);
  else renderTable(list);
  renderSortArrows();

  emptyEl.hidden = list.length > 0;
  if (!list.length) {
    emptyText.textContent = !loaded
      ? "Loading apps…"
      : query.trim()
        ? `No apps match “${query.trim()}”.`
        : "Nothing here yet.";
  }
  applySelection();
}

// --- keyboard navigation ---------------------------------------------------
function scrollToId(id) {
  document.querySelector(`[data-id="${CSS.escape(id)}"]`)?.scrollIntoView({ block: "nearest" });
}
function moveSelection(delta) {
  if (!order.length) return;
  const index = order.findIndex((app) => app.id === selectedId);
  let next = index < 0 ? (delta > 0 ? 0 : order.length - 1) : index + delta;
  next = Math.max(0, Math.min(order.length - 1, next));
  selectedId = order[next].id;
  applySelection();
  scrollToId(selectedId);
}
function activateSelected() {
  const app = order.find((a) => a.id === selectedId);
  if (app) primaryAction(app);
}

// --- detail drawer ---------------------------------------------------------
function capabilityChips(app) {
  const wrap = document.createElement("div");
  wrap.className = "chips";
  for (const cap of app.capabilities ?? []) {
    const chip = document.createElement("span");
    chip.className = "chip accent";
    chip.textContent = cap;
    wrap.append(chip);
  }
  for (const perm of app.permissions ?? []) {
    const chip = document.createElement("span");
    chip.className = "chip";
    chip.textContent = perm;
    wrap.append(chip);
  }
  return wrap;
}

function kvList(rows) {
  const dl = document.createElement("dl");
  dl.className = "kv";
  for (const [key, value] of rows) {
    if (!value) continue;
    const dt = document.createElement("dt");
    dt.textContent = key;
    const dd = document.createElement("dd");
    dd.textContent = value;
    dl.append(dt, dd);
  }
  return dl;
}

function cliCommand(app) {
  if (app.self) return "npm run nucleus";
  if (app.status === "planned" || !app.launchable) return null;
  return `npm run nucleus -- open ${app.id}`;
}

function buildDetail(app) {
  const nodes = [];

  const close = document.createElement("button");
  close.className = "drawer-close";
  close.textContent = "✕";
  close.title = "Close";
  close.addEventListener("click", closeDetail);
  nodes.push(close);

  const head = document.createElement("div");
  head.className = "drawer-head";
  head.append(iconTile(app, "lg", false));
  const headText = document.createElement("div");
  const h2 = document.createElement("h2");
  h2.textContent = app.name;
  const sub = document.createElement("div");
  sub.className = "drawer-sub";
  sub.textContent = app.version === "—"
    ? `${groupTitle(app.category)} · ${app.status}`
    : `v${app.version} · ${groupTitle(app.category)} · ${app.status}`;
  headText.append(h2, sub);
  head.append(headText);
  nodes.push(head);

  if (app.description) {
    const p = document.createElement("p");
    p.className = "drawer-desc";
    p.textContent = app.description;
    nodes.push(p);
  }

  if (app.inspiredBy) {
    const section = document.createElement("div");
    section.className = "drawer-section";
    const h3 = document.createElement("h3");
    h3.textContent = "Inspired by";
    const p = document.createElement("p");
    p.className = "drawer-desc";
    p.textContent = app.inspiredBy;
    section.append(h3, p);
    nodes.push(section);
  }

  if ((app.capabilities ?? []).length || (app.permissions ?? []).length) {
    const section = document.createElement("div");
    section.className = "drawer-section";
    const h3 = document.createElement("h3");
    h3.textContent = "Command bus";
    section.append(h3, capabilityChips(app));
    nodes.push(section);
  }

  const details = document.createElement("div");
  details.className = "drawer-section";
  const h3 = document.createElement("h3");
  h3.textContent = "Details";
  details.append(h3, kvList([
    ["ID", app.id],
    ["Author", app.author],
    ["Source", app.source],
    ["Status", app.status],
    ["Version", app.version === "—" ? "planned" : app.version],
  ]));
  nodes.push(details);

  const command = cliCommand(app);
  if (command) {
    const section = document.createElement("div");
    section.className = "drawer-section";
    const h3 = document.createElement("h3");
    h3.textContent = "Run from CLI";
    const line = document.createElement("div");
    line.className = "copy-line";
    const code = document.createElement("span");
    code.textContent = command;
    const copy = document.createElement("button");
    copy.textContent = "Copy";
    copy.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(command);
        copy.textContent = "Copied";
        setTimeout(() => { copy.textContent = "Copy"; }, 1200);
      } catch {
        copy.textContent = "Failed";
      }
    });
    line.append(code, copy);
    section.append(h3, line);
    nodes.push(section);
  }

  const actions = document.createElement("div");
  actions.className = "drawer-actions";
  actions.append(actionButton(app));
  const closeBtn = document.createElement("button");
  closeBtn.className = "btn subtle";
  closeBtn.textContent = "Close";
  closeBtn.addEventListener("click", closeDetail);
  actions.append(closeBtn);
  nodes.push(actions);

  return nodes;
}

function openDetail(id) {
  const app = apps.find((a) => a.id === id);
  if (!app) return;
  detailId = id;
  drawerPanel.replaceChildren(...buildDetail(app));
  drawerEl.hidden = false;
}
function closeDetail() {
  detailId = null;
  drawerEl.hidden = true;
}
drawerEl.addEventListener("mousedown", (event) => { if (event.target === drawerEl) closeDetail(); });

// --- command palette -------------------------------------------------------
function paletteEntries() {
  const items = [];
  for (const app of apps) {
    items.push({
      label: app.name,
      kind: app.status === "planned" ? "soon" : "app",
      hint: `${groupTitle(app.category)} · ${app.version}`,
      run: () => primaryAction(app),
    });
    for (const cap of app.capabilities ?? []) {
      items.push({ label: cap, kind: "command", hint: app.name, run: () => openDetail(app.id) });
    }
  }
  items.push({ label: "Toggle theme", kind: "action", hint: "appearance", run: toggleTheme });
  items.push({
    label: `Switch to ${view === "grid" ? "table" : "grid"} view`,
    kind: "action",
    hint: "view",
    run: () => applyView(view === "grid" ? "table" : "grid"),
  });
  items.push({ label: "Focus search", kind: "action", hint: "search", run: () => { closePalette(); searchEl.focus(); } });
  return items;
}

function updatePaletteActive() {
  const rows = paletteList.querySelectorAll(".palette-item");
  rows.forEach((el, i) => el.classList.toggle("active", i === paletteIndex));
  paletteList.querySelector(".palette-item.active")?.scrollIntoView({ block: "nearest" });
}

function renderPalette() {
  const needle = paletteQuery.trim().toLowerCase();
  const all = paletteEntries();
  const filtered = needle
    ? all.filter((item) => `${item.label} ${item.hint} ${item.kind}`.toLowerCase().includes(needle))
    : all;
  paletteItems = filtered.slice(0, 80);
  if (paletteIndex >= paletteItems.length) paletteIndex = Math.max(0, paletteItems.length - 1);

  const frag = document.createDocumentFragment();
  paletteItems.forEach((item, index) => {
    const li = document.createElement("li");
    li.className = `palette-item${index === paletteIndex ? " active" : ""}`;
    const label = document.createElement("span");
    label.className = "pi-label";
    label.textContent = item.label;
    const kind = document.createElement("span");
    kind.className = "pi-kind";
    kind.textContent = item.kind;
    const hint = document.createElement("span");
    hint.className = "pi-hint";
    hint.textContent = item.hint ?? "";
    li.append(label, kind, hint);
    li.addEventListener("mouseenter", () => { paletteIndex = index; updatePaletteActive(); });
    li.addEventListener("click", () => runPalette(index));
    frag.append(li);
  });

  if (!paletteItems.length) {
    const li = document.createElement("li");
    li.className = "palette-empty";
    li.textContent = "No matching commands";
    frag.append(li);
  }

  paletteList.replaceChildren(frag);
}

function runPalette(index) {
  const item = paletteItems[index];
  if (!item) return;
  closePalette();
  item.run();
}

function openPalette() {
  paletteActive = true;
  paletteQuery = "";
  paletteIndex = 0;
  paletteInput.value = "";
  paletteEl.hidden = false;
  renderPalette();
  paletteInput.focus();
}
function closePalette() {
  paletteActive = false;
  paletteEl.hidden = true;
}
$("#cmd").addEventListener("click", openPalette);
paletteInput.addEventListener("input", () => {
  paletteQuery = paletteInput.value;
  paletteIndex = 0;
  renderPalette();
});
paletteEl.addEventListener("mousedown", (event) => { if (event.target === paletteEl) closePalette(); });

// --- wiring ----------------------------------------------------------------
searchEl.addEventListener("input", () => {
  query = searchEl.value;
  render();
});

document.addEventListener("keydown", (event) => {
  const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement;

  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    paletteActive ? closePalette() : openPalette();
    return;
  }

  if (paletteActive) {
    if (event.key === "Escape") { event.preventDefault(); closePalette(); return; }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      paletteIndex = Math.min(paletteIndex + 1, paletteItems.length - 1);
      updatePaletteActive();
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      paletteIndex = Math.max(paletteIndex - 1, 0);
      updatePaletteActive();
      return;
    }
    if (event.key === "Enter") { event.preventDefault(); runPalette(paletteIndex); return; }
    return;
  }

  if (event.key === "Escape") {
    if (detailId) { closeDetail(); return; }
    if (query) { searchEl.value = ""; query = ""; render(); return; }
    return;
  }

  if (typing) return;
  if (detailId) return;

  if (event.key === "/") { event.preventDefault(); searchEl.focus(); return; }
  if (event.key === "ArrowDown") { event.preventDefault(); moveSelection(1); return; }
  if (event.key === "ArrowUp") { event.preventDefault(); moveSelection(-1); return; }
  if (event.key === "Enter") { event.preventDefault(); activateSelected(); return; }
});

applyView(view);
loadApps();
timer = setInterval(loadApps, 4000);
window.addEventListener("beforeunload", () => clearInterval(timer));
