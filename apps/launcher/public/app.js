// Nucleus launcher UI.

const $ = (sel) => document.querySelector(sel);
const gridEl = $("#grid");
const tableWrap = $("#table-wrap");
const tableBody = $("#table-body");
const emptyEl = $("#empty");
const statusEl = $("#status");
const countEl = $("#count");
const runningCountEl = $("#running-count");
const searchEl = $("#search");
const root = document.documentElement;

let apps = [];
let view = localStorage.getItem("nucleus.view") ?? "grid";
let query = "";
const pending = new Set();
let timer = null;

const setStatus = (msg) => { statusEl.textContent = msg; };

// --- theme -----------------------------------------------------------------
function applyTheme(theme) {
  root.dataset.theme = theme;
  $("#theme").textContent = theme === "dark" ? "☾" : "☀";
  localStorage.setItem("nucleus.theme", theme);
}
applyTheme(localStorage.getItem("nucleus.theme") ?? "dark");
$("#theme").addEventListener("click", () => {
  applyTheme(root.dataset.theme === "dark" ? "light" : "dark");
});

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

// --- data ------------------------------------------------------------------
async function loadApps() {
  try {
    const response = await fetch("/api/apps");
    const payload = await response.json();
    if (!payload.ok) throw new Error(payload.error?.message ?? "failed");
    apps = payload.apps ?? [];
    render();
  } catch (error) {
    setStatus(`Could not reach the launcher: ${error.message}`);
  }
}

function visibleApps() {
  const needle = query.trim().toLowerCase();
  if (!needle) return apps;
  return apps.filter((app) =>
    [app.name, app.id, app.description, app.category, app.inspiredBy, app.status]
      .filter(Boolean)
      .join(" ")
      .toLowerCase()
      .includes(needle));
}

async function openApp(app) {
  pending.add(app.id);
  setStatus(`Starting ${app.name}…`);
  render();
  try {
    const response = await fetch(`/api/apps/${encodeURIComponent(app.id)}/open`, { method: "POST" });
    const payload = await response.json();
    if (payload.apps) apps = payload.apps;
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
    if (payload.apps) apps = payload.apps;
    setStatus(`${app.name} stopped.`);
  } finally {
    pending.delete(app.id);
    render();
  }
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
  tile.className = `tile tile-${size}${withDot && app.running ? " has-dot" : ""}`;
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

function groupOf(list) {
  const ready = list.filter((a) => a.status !== "planned").sort((a, b) => {
    if (a.self !== b.self) return a.self ? 1 : -1;
    if (a.running !== b.running) return a.running ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  const planned = list.filter((a) => a.status === "planned").sort((a, b) => a.name.localeCompare(b.name));
  return [
    { key: "apps", title: "Apps", items: ready },
    { key: "soon", title: "Coming soon", items: planned },
  ].filter((g) => g.items.length > 0);
}

// --- rendering -------------------------------------------------------------
function cardFor(app) {
  const card = document.createElement("article");
  card.className = "card";
  if (app.status === "planned") card.classList.add("planned");
  if (app.running && !app.self) card.classList.add("running");

  const head = document.createElement("div");
  head.className = "card-head";
  head.append(iconTile(app, "lg", true));
  const title = document.createElement("div");
  title.className = "card-title";
  const name = document.createElement("strong");
  name.textContent = app.name;
  const meta = document.createElement("span");
  meta.className = "card-meta";
  meta.textContent = app.version === "—" ? app.category : `v${app.version} · ${app.category}`;
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

function renderTable(list) {
  const frag = document.createDocumentFragment();
  for (const app of list) {
    const row = document.createElement("tr");
    if (app.status === "planned") row.classList.add("planned");

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
    inspired.textContent = app.inspiredBy || "—";
    const category = document.createElement("td");
    category.textContent = app.category;

    const statusCell = document.createElement("td");
    statusCell.append(statusPill(app.status, false));

    const version = document.createElement("td");
    version.textContent = app.version;

    const actionCell = document.createElement("td");
    actionCell.className = "right";
    actionCell.append(actionButton(app));

    row.append(appCell, inspired, category, statusCell, version, actionCell);
    frag.appendChild(row);
  }
  tableBody.replaceChildren(frag);
}

function render() {
  const list = visibleApps();
  const ready = apps.filter((a) => a.status !== "planned").length;
  const planned = apps.filter((a) => a.status === "planned").length;
  const running = apps.filter((a) => a.running && !a.self).length;

  countEl.textContent = `${ready} apps · ${planned} planned`;
  runningCountEl.textContent = running > 0 ? `● ${running} running` : "";
  emptyEl.hidden = list.length > 0;

  if (view === "grid") renderGrid(list);
  else renderTable(list);
}

// --- wiring ----------------------------------------------------------------
searchEl.addEventListener("input", () => {
  query = searchEl.value;
  render();
});

document.addEventListener("keydown", (event) => {
  if (event.key === "/" && document.activeElement !== searchEl) {
    event.preventDefault();
    searchEl.focus();
  }
});

applyView(view);
loadApps();
timer = setInterval(loadApps, 4000);
window.addEventListener("beforeunload", () => clearInterval(timer));
