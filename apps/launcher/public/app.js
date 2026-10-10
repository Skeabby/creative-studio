// Creative Studio launcher UI.

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
let view = localStorage.getItem("studio.view") ?? "grid";
let query = "";
const pending = new Set();
let timer = null;

const setStatus = (msg) => { statusEl.textContent = msg; };

// --- theme -----------------------------------------------------------------
function applyTheme(theme) {
  root.dataset.theme = theme;
  $("#theme").textContent = theme === "dark" ? "☾" : "☀";
  localStorage.setItem("studio.theme", theme);
}
applyTheme(localStorage.getItem("studio.theme") ?? "dark");
$("#theme").addEventListener("click", () => {
  applyTheme(root.dataset.theme === "dark" ? "light" : "dark");
});

// --- view toggle -----------------------------------------------------------
function applyView(next) {
  view = next;
  localStorage.setItem("studio.view", next);
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

// --- rendering -------------------------------------------------------------
function statusPill(status) {
  const span = document.createElement("span");
  span.className = `pill pill-${status}`;
  span.textContent = status;
  return span;
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
    button.textContent = "Planned";
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
    button.className = "btn";
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

function iconTile(app, size) {
  const tile = document.createElement("div");
  tile.className = `tile tile-${size}`;
  if (app.iconUrl) {
    const img = document.createElement("img");
    img.src = app.iconUrl;
    img.alt = "";
    tile.appendChild(img);
  } else {
    tile.style.background = app.color ?? "#5b8cff";
    tile.textContent = app.emoji ?? "✨";
  }
  return tile;
}

function renderGrid(list) {
  const frag = document.createDocumentFragment();
  for (const app of list) {
    const card = document.createElement("article");
    card.className = "card";
    if (app.status === "planned") card.classList.add("planned");
    if (app.running) card.classList.add("running");

    const head = document.createElement("div");
    head.className = "card-head";
    head.append(iconTile(app, "lg"));
    const title = document.createElement("div");
    title.className = "card-title";
    const name = document.createElement("strong");
    name.textContent = app.name;
    const meta = document.createElement("span");
    meta.className = "card-meta";
    meta.textContent = app.version === "—" ? app.category : `v${app.version} · ${app.category}`;
    title.append(name, meta);
    head.append(title);
    head.append(statusPill(app.status));

    const description = document.createElement("p");
    description.className = "card-desc";
    description.textContent = app.description || "No description yet.";

    const foot = document.createElement("div");
    foot.className = "card-foot";
    if (app.inspiredBy) {
      const inspired = document.createElement("span");
      inspired.className = "muted";
      inspired.textContent = `like ${app.inspiredBy}`;
      foot.append(inspired);
    }
    foot.append(actionButton(app));

    card.append(head, description, foot);
    frag.appendChild(card);
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
    appWrap.append(iconTile(app, "sm"));
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
    statusCell.append(statusPill(app.status));

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
  const running = apps.filter((a) => a.running && !a.self).length;

  countEl.textContent = `${apps.filter((a) => a.status !== "planned").length} apps · ${apps.filter((a) => a.status === "planned").length} planned`;
  runningCountEl.textContent = running > 0 ? `${running} running` : "";
  emptyEl.hidden = list.length > 0;

  if (view === "grid") renderGrid(list);
  else renderTable(list);
}

// --- wiring ----------------------------------------------------------------
searchEl.addEventListener("input", () => {
  query = searchEl.value;
  render();
});

applyView(view);
loadApps();
timer = setInterval(loadApps, 4000);
window.addEventListener("beforeunload", () => clearInterval(timer));
