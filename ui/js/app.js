import { invoke, listen, fileSrc, win, isApp } from "./api.js";
import * as nav from "./nav.js";
import { playBoot, PALETTE } from "./boot.js";
import { SECTIONS, TAG_TEXT, ACCENTS } from "./schema.js";

// ───────────────────────── tiny DOM helper (text is always textContent, never HTML) ─────────────────────────
function h(tag, attrs = {}, ...kids) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v === undefined || v === null || v === false) continue;
    if (k === "class") el.className = v;
    else if (k === "style" && typeof v === "object") Object.assign(el.style, v);
    else if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (k === "html") el.innerHTML = v; // only used with static SVG strings below
    else if (v === true) el.setAttribute(k, "");
    else el.setAttribute(k, v);
  }
  for (const kid of kids.flat()) {
    if (kid === null || kid === undefined || kid === false) continue;
    el.append(kid instanceof Node ? kid : document.createTextNode(String(kid)));
  }
  return el;
}
const $ = (s, r = document) => r.querySelector(s);
const ICON = {
  play: '<svg viewBox="0 0 14 14"><path d="M3 1.8v10.4c0 .5.6.8 1 .5l8.1-5.2c.4-.2.4-.8 0-1L4 1.3c-.4-.3-1 0-1 .5z"/></svg>',
  stop: '<svg viewBox="0 0 14 14"><rect x="2.5" y="2.5" width="9" height="9" rx="1.5"/></svg>',
  star: '<svg viewBox="0 0 24 24"><path d="M12 3.5l2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8-4.3-4.1 5.9-.9z"/></svg>',
  sliders: '<svg viewBox="0 0 24 24"><path d="M4 7h10M18 7h2M4 17h4M12 17h8"/><circle cx="16" cy="7" r="2"/><circle cx="10" cy="17" r="2"/></svg>',
  info: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8.5"/><path d="M12 11v5M12 8h.01"/></svg>',
  folder: '<svg viewBox="0 0 24 24"><path d="M3.5 7.5a2 2 0 0 1 2-2h4l2 2h7a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/></svg>',
  plus: '<svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>',
  x: '<svg viewBox="0 0 12 12"><path d="M1 1l10 10M11 1L1 11"/></svg>',
  min: '<svg viewBox="0 0 10 10"><path d="M0 5h10"/></svg>',
  max: '<svg viewBox="0 0 10 10"><rect x=".5" y=".5" width="9" height="9"/></svg>',
  close: '<svg viewBox="0 0 10 10"><path d="M0 0l10 10M10 0L0 10"/></svg>',
};
const MARK = (a = "var(--a1)", b = "var(--a2)", c = "var(--fg)") =>
  `<svg viewBox="0 0 64 64"><rect x="14" y="2.5" width="42" height="9" rx="4.5" fill="${a}"/><rect x="8" y="15" width="22" height="9" rx="4.5" fill="${b}"/><rect x="14" y="27.5" width="36" height="9" rx="4.5" fill="${c}"/><rect x="34" y="40" width="22" height="9" rx="4.5" fill="${b}"/><rect x="8" y="52.5" width="42" height="9" rx="4.5" fill="${a}"/></svg>`;

// ───────────────────────── state ─────────────────────────
const S = {
  boot: null,
  settings: null,
  games: [],
  sys: null,
  profile: null,
  engine: { path: null, version: null, error: null },
  running: null,
  view: "home",
  sel: 0,
  lib: { q: "", filter: "all", layout: "grid" },
  section: "profile",
  perGame: null, // game id when editing per-game settings
  logSeq: 0,
  devFilter: { info: true, warn: true, error: true, debug: true, source: "all", q: "", follow: true },
  checks: null,
  download: null,
  sizes: {},
};

const VIEWS = ["home", "library", "settings", "dev"];
const visibleViews = () => VIEWS.filter((v) => v !== "dev" || S.settings?.dev.enabled);

// ───────────────────────── helpers ─────────────────────────
const get = (obj, path) => path.split(".").reduce((o, k) => (o == null ? o : o[k]), obj);
function set(obj, path, val) {
  const ks = path.split(".");
  let o = obj;
  for (const k of ks.slice(0, -1)) o = o[k];
  o[ks.at(-1)] = val;
}
const fmtTime = (secs) => {
  if (!secs) return "Never played";
  const hrs = Math.floor(secs / 3600), m = Math.floor((secs % 3600) / 60);
  return hrs ? `${hrs} h ${m} min` : m ? `${m} min` : `${secs} s`;
};
const fmtSize = (mb) => (!mb ? "—" : mb >= 1024 ? (mb / 1024).toFixed(1) + " GB" : mb + " MB");
const fmtAgo = (sec) => {
  if (!sec) return "—";
  const d = Date.now() / 1000 - sec;
  if (d < 90) return "just now";
  if (d < 3600) return Math.round(d / 60) + " min ago";
  if (d < 86400) return Math.round(d / 3600) + " h ago";
  return Math.round(d / 86400) + " days ago";
};
const entry = (id) => S.settings.games[id] || (S.settings.games[id] = { favorite: false, last_played: null, play_seconds: 0, launches: 0, status: "unknown", note: "", emu: null, perf: null });
const initials = (t) => t.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]).join("").toUpperCase();
const STATUS = { unknown: "Untested", boots: "Boots", menus: "Menus", playable: "Playable", broken: "Broken" };

function log(level, msg) { invoke("ui_log", { level, msg }).catch(() => {}); }

let saveTimer = null;
function save(immediate = false) {
  clearTimeout(saveTimer);
  const run = async () => {
    try {
      S.settings = await invoke("save_settings", { settings: S.settings });
    } catch (e) { toast(String(e), "err"); }
  };
  if (immediate) return run();
  saveTimer = setTimeout(run, 350);
}

function toast(msg, kind = "") {
  const t = h("div", { class: "toast " + kind }, h("div", {}, msg));
  $("#toasts").append(t);
  setTimeout(() => { t.style.transition = "opacity .4s"; t.style.opacity = "0"; }, kind === "err" ? 7000 : 3800);
  setTimeout(() => t.remove(), kind === "err" ? 7500 : 4300);
  if (kind === "err") nav.blip("err");
}

function applyTheme() {
  const u = S.settings.ui;
  document.documentElement.style.setProperty("--a1", PALETTE[u.accent1] || PALETTE.ember);
  document.documentElement.style.setProperty("--a2", PALETTE[u.accent2] || PALETTE.cobalt);
  document.body.classList.toggle("reduce-motion", u.reduce_motion);
  nav.setSounds(u.ui_sounds);
}

function sortedGames() {
  return [...S.games].sort((a, b) => {
    const ea = S.settings.games[a.id] || {}, eb = S.settings.games[b.id] || {};
    return (eb.last_played || 0) - (ea.last_played || 0) || a.title.localeCompare(b.title);
  });
}

// ───────────────────────── top bar ─────────────────────────
function renderTop() {
  const bar = $("#topbar");
  bar.replaceChildren();
  const brand = h("div", { class: "brand", "data-tauri-drag-region": true },
    h("span", { html: MARK() }), h("span", { class: "word", "data-tauri-drag-region": true }, "STRATA"));
  const tabs = h("nav", { class: "tabs" }, visibleViews().map((v, i) =>
    h("button", { class: "tab" + (S.view === v ? " active" : ""), "data-nav": true, id: "tab-" + v, onclick: () => go(v) },
      { home: "Home", library: "Library", settings: "Settings", dev: "Developer" }[v])));
  const gpu = S.profile;
  const chip = h("button", {
    class: "chip " + (!gpu ? "" : gpu.tier === "unsupported" ? "bad" : S.engine.path ? "ok" : "warn"),
    "data-nav": true,
    title: "Hardware profile",
    onclick: () => { S.section = "profile"; S.perGame = null; go("settings"); },
  }, h("span", { class: "dot" }), gpu ? (gpu.gpu_name ? shortGpu(gpu.gpu_name) + " · " : "") + gpu.label : "Checking PC…");
  const status = h("div", { class: "status", "data-tauri-drag-region": true },
    chip,
    nav.pad.connected ? h("span", { class: "chip", title: nav.pad.name }, h("span", { class: "dot", style: { background: "var(--a2)" } }), "Controller") : null,
    h("span", { class: "clock", id: "clock", "data-tauri-drag-region": true }, clock()),
    isApp ? h("div", { class: "winctl" },
      h("button", { title: "Minimize", onclick: win.minimize, html: ICON.min }),
      h("button", { title: "Maximize", onclick: win.toggleMax, html: ICON.max }),
      h("button", { class: "close", title: "Close", onclick: win.close, html: ICON.close })) : h("span", { style: { width: "18px" } }));
  bar.append(brand, tabs, status);
}
const shortGpu = (n) => n.replace(/NVIDIA |GeForce |AMD |Radeon\(TM\) |Intel\(R\) |\(R\)|\(TM\)| Laptop GPU| Graphics/g, "").trim();
function clock() {
  const d = new Date();
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: !S.settings?.ui.clock_24h });
}

function renderHints() {
  const k = (key, text) => h("span", {}, h("span", { class: "k" }, key), text);
  const items = {
    home: [k("↵", "Play"), k("I", "Details"), k("F", "Favorite"), k("S", "Game settings")],
    library: [k("↵", "Open"), k("/", "Search"), k("F", "Favorite")],
    settings: [k("↵", "Change"), k("←→", "Adjust"), k("Esc", "Back")],
    dev: [k("T", "Run tests"), k("C", "Clear log")],
  }[S.view] || [];
  $("#hints").replaceChildren(...items, h("span", { class: "right" }, h("span", {}, h("span", { class: "k" }, "Q"), h("span", { class: "k" }, "E"), "Switch tab"), k("F11", "Fullscreen")));
}

function go(view) {
  if (!visibleViews().includes(view)) view = "home";
  S.view = view;
  document.querySelectorAll(".view").forEach((v) => v.classList.toggle("active", v.id === "view-" + view));
  renderTop();
  renderHints();
  if (view === "home") renderHome();
  if (view === "library") renderLibrary();
  if (view === "settings") renderSettings();
  if (view === "dev") renderDev();
  requestAnimationFrame(() => {
    const first = view === "home" ? $(".tile.sel") || $("#home [data-nav]") : $(`#view-${view} [data-nav]`);
    if (first) nav.focus(first, { silent: true });
  });
}

// ───────────────────────── home ─────────────────────────
let bgSwap = 0;
function setBackdrop(url) {
  const holder = $("#home .backdrop");
  const imgs = holder.querySelectorAll(".img");
  const next = imgs[bgSwap % 2], prevImg = imgs[(bgSwap + 1) % 2];
  bgSwap++;
  if (url) {
    next.style.backgroundImage = `url("${url}")`;
    next.classList.add("show");
  } else next.classList.remove("show");
  prevImg.classList.remove("show");
  holder.querySelector(".stripes").style.opacity = url ? "0" : "0.55";
}

function renderHome() {
  const root = $("#home");
  const games = sortedGames();
  const content = root.querySelector(".content");
  content.replaceChildren();
  if (!games.length) {
    setBackdrop(null);
    content.append(h("div", { class: "empty" },
      h("div", { class: "micro" }, "Library · 0 games"),
      h("h1", {}, "Your shelf is empty."),
      h("p", {}, "Copy a game folder (the one that contains eboot.bin) into your games folder, or add the folder where your games already are. Strata finds them automatically."),
      h("code", { class: "path" }, S.boot.games_dir),
      h("div", { class: "actions", style: { display: "flex", gap: "12px" } },
        h("button", { class: "btn primary", "data-nav": true, onclick: () => invoke("open_path", { path: S.boot.games_dir }) }, "Open games folder"),
        h("button", { class: "btn", "data-nav": true, onclick: addFolder }, "Add a folder"),
        h("button", { class: "btn ghost", "data-nav": true, onclick: refresh }, "Refresh"))));
    return;
  }
  S.sel = Math.min(S.sel, games.length - 1);
  const g = games[S.sel];
  const e = entry(g.id);
  setBackdrop(g.background ? fileSrc(g.background) : null);
  const isRunning = S.running && S.running.game_id === g.id;

  const hero = h("div", { class: "hero" },
    h("div", { class: "eyebrow" },
      h("span", { class: "micro idx" }, String(S.sel + 1).padStart(2, "0") + " / " + String(games.length).padStart(2, "0")),
      h("span", { class: "micro" }, g.archive ? "Archive" : g.title_id || "Folder")),
    h("h1", {}, g.title),
    h("div", { class: "meta" },
      h("span", {}, "Played ", h("b", {}, fmtTime(e.play_seconds))),
      h("span", {}, "Last ", h("b", {}, fmtAgo(e.last_played))),
      g.version ? h("span", {}, "Version ", h("b", {}, g.version)) : null,
      h("span", {}, "Status ", h("b", {}, STATUS[e.status] || "Untested")),
      e.emu ? h("span", { class: "micro", style: { color: "var(--a1)" } }, "Custom settings") : null),
    h("div", { class: "actions" },
      isRunning
        ? h("button", { class: "btn primary", id: "play", "data-nav": true, onclick: stopGame, html: ICON.stop + "<span>Stop</span>" })
        : h("button", { class: "btn primary", id: "play", "data-nav": true, onclick: () => launch(g.id), html: ICON.play + "<span>Play</span>", disabled: !!S.running }),
      h("button", { class: "btn iconbtn", "data-nav": true, title: "Details", onclick: () => openDrawer(g.id), html: ICON.info }),
      h("button", { class: "btn iconbtn" + (e.favorite ? " on" : ""), "data-nav": true, title: "Favorite", onclick: () => toggleFav(g.id), html: ICON.star }),
      h("button", { class: "btn iconbtn", "data-nav": true, title: "Game settings", onclick: () => openGameSettings(g.id), html: ICON.sliders })));

  const shelf = h("div", { class: "shelf", id: "shelf" });
  games.forEach((x, i) => {
    const xe = S.settings.games[x.id] || {};
    shelf.append(h("button", {
      class: "tile" + (i === S.sel ? " sel" : ""), "data-nav": true, id: "tile-" + i,
      "data-nav-up": "#play",
      onclick: () => { if (i === S.sel) launch(x.id); else selectTile(i); },
      onnavfocus: () => { if (i !== S.sel) selectTile(i, true); },
    },
      h("div", { class: "cover" },
        x.icon ? h("img", { src: fileSrc(x.icon), alt: "", loading: "lazy" }) : h("div", { class: "ph" }, initials(x.title)),
        h("span", { class: "num" }, String(i + 1).padStart(2, "0")),
        xe.favorite ? h("span", { class: "fav" }) : null),
      h("div", { class: "label" }, x.title)));
  });
  shelf.append(h("button", { class: "tile add", "data-nav": true, "data-nav-up": "#play", onclick: addFolder, title: "Add a folder" },
    h("div", { class: "cover", html: ICON.plus }), h("div", { class: "label" }, "Add folder")));

  content.append(hero, h("div", { class: "shelf-wrap" },
    h("div", { class: "shelf-head" }, h("span", { class: "micro" }, "Recently played"), h("span", { class: "micro" }, `${games.length} in library`)),
    shelf));
  requestAnimationFrame(() => $("#tile-" + S.sel)?.scrollIntoView({ inline: "center", block: "nearest" }));
}

function selectTile(i, fromFocus = false) {
  S.sel = i;
  const focusedId = document.activeElement?.id;
  renderHome();
  const target = fromFocus ? $("#" + (focusedId || "tile-" + i)) : $("#tile-" + i);
  if (target) nav.focus(target, { silent: true });
}

// ───────────────────────── library ─────────────────────────
function filteredGames() {
  const q = S.lib.q.trim().toLowerCase();
  return [...S.games]
    .filter((g) => !q || g.title.toLowerCase().includes(q) || g.title_id.toLowerCase().includes(q))
    .filter((g) => {
      const e = S.settings.games[g.id] || {};
      if (S.lib.filter === "fav") return e.favorite;
      if (S.lib.filter === "recent") return e.last_played;
      if (S.lib.filter === "playable") return e.status === "playable";
      return true;
    })
    .sort((a, b) => S.lib.filter === "recent"
      ? (S.settings.games[b.id]?.last_played || 0) - (S.settings.games[a.id]?.last_played || 0)
      : a.title.localeCompare(b.title));
}

function renderLibrary() {
  const root = $("#view-library .page");
  const list = filteredGames();
  const seg = (key, opts) => h("div", { class: "seg" }, opts.map(([v, label]) =>
    h("button", { class: S.lib[key] === v ? "on" : "", "data-nav": true, onclick: () => { S.lib[key] = v; renderLibrary(); } }, label)));
  const search = h("input", { class: "search", id: "search", placeholder: "Search title or ID", value: S.lib.q, "data-nav": true, spellcheck: "false" });
  search.addEventListener("input", () => { S.lib.q = search.value; renderLibraryBody(); });

  root.replaceChildren(
    h("div", { class: "page-head" },
      h("div", {}, h("div", { class: "micro" }, `${S.games.length} games · ${S.settings.library_folders.length} folders`), h("h2", {}, "Library")),
      h("div", { class: "tools" },
        search,
        seg("filter", [["all", "All"], ["fav", "Favorites"], ["recent", "Recent"], ["playable", "Playable"]]),
        seg("layout", [["grid", "Grid"], ["list", "List"]]),
        h("button", { class: "btn small", "data-nav": true, onclick: refresh }, "Refresh"))),
    h("div", { class: "scroll", id: "libbody" }));
  renderLibraryBody(list);
}

function renderLibraryBody(list = filteredGames()) {
  const body = $("#libbody");
  if (!body) return;
  body.replaceChildren();
  if (!list.length) {
    body.append(h("div", { class: "banner" }, S.games.length ? "Nothing matches." : "No games yet. Add a folder in Settings → Library.",
      h("button", { class: "btn small", "data-nav": true, onclick: addFolder }, "Add folder")));
    return;
  }
  if (S.lib.layout === "grid") {
    body.append(h("div", { class: "grid" }, list.map((g) => {
      const e = S.settings.games[g.id] || {};
      return h("button", { class: "card", "data-nav": true, onclick: () => openDrawer(g.id), "data-id": g.id },
        h("div", { class: "cover" },
          g.icon ? h("img", { src: fileSrc(g.icon), alt: "", loading: "lazy" }) : h("div", { class: "ph" }, initials(g.title)),
          e.status && e.status !== "unknown" ? h("span", { class: "badge " + e.status }, STATUS[e.status]) : null),
        h("div", { class: "t" }, g.title),
        h("div", { class: "s micro" }, g.archive ? "ZAR archive" : g.title_id || "—"));
    })));
  } else {
    body.append(h("div", { class: "list" },
      h("div", { class: "row head" }, h("span"), h("span", {}, "Title"), h("span", {}, "ID"), h("span", {}, "Version"), h("span", {}, "Played"), h("span", {}, "Status")),
      list.map((g) => {
        const e = S.settings.games[g.id] || {};
        return h("button", { class: "row", "data-nav": true, onclick: () => openDrawer(g.id), "data-id": g.id },
          g.icon ? h("img", { src: fileSrc(g.icon), alt: "" }) : h("span", { class: "ph" }, initials(g.title)),
          h("span", { class: "t" }, g.title), h("span", { class: "mono" }, g.title_id || "—"), h("span", { class: "mono" }, g.version || "—"),
          h("span", {}, fmtTime(e.play_seconds)), h("span", {}, STATUS[e.status] || "Untested"));
      })));
  }
}

// ───────────────────────── drawer ─────────────────────────
let drawerFor = null;
function openDrawer(id) {
  const g = S.games.find((x) => x.id === id);
  if (!g) return;
  drawerFor = id;
  const e = entry(id);
  const d = $("#drawer");
  const statusRow = h("div", { class: "statuses" }, Object.entries(STATUS).map(([k, label]) =>
    h("button", { class: e.status === k ? "on" : "", "data-nav": true, onclick: (ev) => {
      e.status = k; save();
      ev.currentTarget.parentElement.querySelectorAll("button").forEach((b) => b.classList.remove("on"));
      ev.currentTarget.classList.add("on");
    } }, label)));
  const note = h("textarea", { class: "note", placeholder: "Notes — what works, which settings helped…", "data-nav": true }, e.note || "");
  note.addEventListener("input", () => { e.note = note.value; save(); });
  const sizeEl = h("div", { class: "v" }, S.sizes[id] ? fmtSize(S.sizes[id]) : "Measuring…");
  if (!S.sizes[id]) invoke("game_size", { path: g.path }).then((mb) => { S.sizes[id] = mb; sizeEl.textContent = fmtSize(mb); }).catch(() => (sizeEl.textContent = "—"));
  const isRunning = S.running && S.running.game_id === id;

  d.replaceChildren(
    h("button", { class: "close-x", "data-nav": true, onclick: closeDrawer, html: ICON.x, title: "Close" }),
    h("div", { class: "art", style: g.background ? { backgroundImage: `url("${fileSrc(g.background)}")` } : {} },
      h("div", { class: "icon", style: g.icon ? { backgroundImage: `url("${fileSrc(g.icon)}")` } : {} })),
    h("div", { class: "body" },
      h("div", { class: "micro" }, g.archive ? "ZAR archive" : g.title_id || "Game folder"),
      h("h3", {}, g.title),
      h("div", { class: "actions", style: { marginTop: "22px" } },
        isRunning
          ? h("button", { class: "btn primary", "data-nav": true, onclick: stopGame, html: ICON.stop + "<span>Stop</span>" })
          : h("button", { class: "btn primary", "data-nav": true, disabled: !!S.running, onclick: () => { closeDrawer(); launch(id); }, html: ICON.play + "<span>Play</span>" }),
        h("button", { class: "btn iconbtn" + (e.favorite ? " on" : ""), "data-nav": true, title: "Favorite", html: ICON.star, onclick: (ev) => { toggleFav(id); ev.currentTarget.classList.toggle("on", entry(id).favorite); } }),
        h("button", { class: "btn", "data-nav": true, onclick: () => { closeDrawer(); openGameSettings(id); } }, e.emu ? "Custom settings" : "Game settings"),
        h("button", { class: "btn iconbtn", "data-nav": true, title: "Open folder", html: ICON.folder, onclick: () => invoke("open_path", { path: g.archive ? g.folder : g.path }).catch((x) => toast(String(x), "err")) })),
      h("div", { class: "facts" },
        h("div", {}, h("div", { class: "micro" }, "Version"), h("div", { class: "v" }, g.version || "—")),
        h("div", {}, h("div", { class: "micro" }, "Needs firmware"), h("div", { class: "v" }, g.firmware || "—")),
        h("div", {}, h("div", { class: "micro" }, "Time played"), h("div", { class: "v" }, fmtTime(e.play_seconds))),
        h("div", {}, h("div", { class: "micro" }, "Launches"), h("div", { class: "v" }, String(e.launches || 0))),
        h("div", {}, h("div", { class: "micro" }, "Size"), sizeEl),
        h("div", {}, h("div", { class: "micro" }, "Last played"), h("div", { class: "v" }, fmtAgo(e.last_played))),
        h("div", { style: { gridColumn: "1 / -1" } }, h("div", { class: "micro" }, "Location"), h("div", { class: "v mono", style: { fontSize: "12px" } }, g.path))),
      h("div", { class: "micro" }, "How does it run?"), statusRow, note));
  $("#scrim").classList.add("show");
  d.classList.add("show");
  nav.setScope(d);
  setTimeout(() => nav.focus(d.querySelector(".btn.primary"), { silent: true }), 60);
}
function closeDrawer() {
  if (!drawerFor) return false;
  const id = drawerFor;
  drawerFor = null;
  $("#drawer").classList.remove("show");
  $("#scrim").classList.remove("show");
  nav.setScope(null);
  if (S.view === "library") {
    renderLibraryBody();
    const card = document.querySelector(`[data-id="${CSS.escape(id)}"]`);
    if (card) nav.focus(card, { silent: true });
  } else if (S.view === "home") renderHome();
  return true;
}

// ───────────────────────── settings ─────────────────────────
function openGameSettings(id) {
  S.perGame = id;
  if (!SECTIONS.find((s) => s.id === S.section)?.perGame) S.section = "display";
  go("settings");
}

function activeEmu() {
  // object whose emu/perf the fields read & write
  if (!S.perGame) return S.settings;
  const e = entry(S.perGame);
  return { emu: e.emu || S.settings.emu, perf: e.perf || S.settings.perf, ui: S.settings.ui, dev: S.settings.dev, _custom: !!e.emu };
}

function renderSettings() {
  const root = $("#view-settings .page");
  const game = S.perGame ? S.games.find((g) => g.id === S.perGame) : null;
  if (S.perGame && !game) S.perGame = null;
  const sections = SECTIONS.filter((s) => !S.perGame || s.perGame);
  if (!sections.find((s) => s.id === S.section)) S.section = sections[0].id;

  let lastGroup = null;
  const list = h("div", { class: "sections" });
  for (const s of sections) {
    if (s.group !== lastGroup && !S.perGame) { list.append(h("div", { class: "micro group" }, s.group)); lastGroup = s.group; }
    list.append(h("button", { class: S.section === s.id ? "on" : "", "data-nav": true, onclick: () => { S.section = s.id; renderSettings(); nav.focus($(`.sections .on`), { silent: true }); } }, h("span", {}, s.title)));
  }

  root.replaceChildren(
    h("div", { class: "page-head" },
      h("div", {}, h("div", { class: "micro" }, game ? "Game settings" : "Preferences"), h("h2", {}, game ? game.title : "Settings")),
      game ? h("div", { class: "tools" }, h("button", { class: "btn small", "data-nav": true, onclick: () => { S.perGame = null; renderSettings(); } }, "Back to global settings")) : null),
    h("div", { class: "settings" }, list, h("div", { class: "pane", id: "pane" })));
  renderPane();
}

function renderPane() {
  const pane = $("#pane");
  if (!pane || S.view !== "settings") return;
  const sec = SECTIONS.find((s) => s.id === S.section);
  pane.replaceChildren(h("h3", {}, sec.title), sec.lead ? h("p", { class: "lead" }, sec.lead) : null);

  if (S.perGame) {
    const e = entry(S.perGame);
    const on = !!e.emu;
    pane.append(h("div", { class: "banner " + (on ? "accent" : "") },
      h("span", {}, on ? "This game uses its own settings. Changes here affect only this game." : "This game follows the global settings. Turn on custom settings to change them for this game only."),
      h("button", { class: "switch" + (on ? " on" : ""), "data-nav": true, onclick: () => {
        if (e.emu) { e.emu = null; e.perf = null; } else { e.emu = structuredClone(S.settings.emu); e.perf = structuredClone(S.settings.perf); }
        save(); renderPane();
      } })));
  }

  const custom = { profile: paneProfile, ram: paneRam, library: paneLibrary, engine: paneEngine, appearance: paneAppearance, about: paneAbout }[sec.custom];
  if (custom) custom(pane);
  if (sec.fields) {
    const target = activeEmu();
    const locked = S.perGame && !target._custom;
    for (const f of sec.fields) pane.append(field(f, target, locked));
  }
  if (sec.id === "display" || sec.id === "rendering") {
    pane.append(h("div", { style: { marginTop: "26px", display: "flex", gap: "10px" } },
      h("button", { class: "btn small", "data-nav": true, onclick: resetSection }, "Reset this page to defaults")));
  }
}

async function resetSection() {
  const sec = SECTIONS.find((s) => s.id === S.section);
  const defs = (await import("./defaults.js")).EMU_DEFAULTS;
  const target = S.perGame ? entry(S.perGame).emu : S.settings.emu;
  if (!target) return;
  for (const f of sec.fields) if (f.key.startsWith("emu.")) target[f.key.slice(4)] = defs[f.key.slice(4)];
  save(); renderPane(); toast("Reset to defaults.", "ok");
}

function field(f, target, locked) {
  const val = get(target, f.key);
  const requiresOff = f.requires && !get(target, f.requires);
  const write = (v) => {
    if (S.perGame && (f.key.startsWith("emu.") || f.key.startsWith("perf."))) {
      const e = entry(S.perGame);
      set({ emu: e.emu, perf: e.perf }, f.key, v);
    } else set(S.settings, f.key, v);
    if (f.key.startsWith("ui.")) applyTheme();
    if (f.key === "dev.enabled") renderTop();
    save();
    if (SECTIONS.find((s) => s.id === S.section)?.fields?.some((x) => x.requires === f.key)) renderPane();
  };
  let ctl;
  switch (f.type) {
    case "toggle":
      ctl = h("button", { class: "switch" + (val ? " on" : ""), "data-nav": true, "aria-pressed": String(!!val), onclick: (ev) => { const nv = !get(activeEmu(), f.key); ev.currentTarget.classList.toggle("on", nv); write(nv); } });
      break;
    case "select": {
      const sel = h("select", { "data-nav": true }, f.options.map((o) => {
        const [v, label] = Array.isArray(o) ? o : [o, o.replace("x", " × ")];
        return h("option", { value: String(v), selected: String(v) === String(val) }, label);
      }));
      if (f.key === "emu.resolution" && !f.options.includes(val)) sel.prepend(h("option", { value: val, selected: true }, val + " (custom)"));
      sel.addEventListener("change", () => write(typeof val === "number" ? Number(sel.value) : sel.value));
      ctl = h("div", { class: "select" }, sel);
      break;
    }
    case "gpu": {
      const gpus = S.sys?.gpus || [];
      const sel = h("select", { "data-nav": true },
        h("option", { value: "-1", selected: val === -1 }, "Automatic"),
        gpus.map((g) => h("option", { value: String(g.index), selected: g.index === val, disabled: !g.vulkan_ok }, `#${g.index} ${shortGpu(g.name)} · ${Math.round(g.vram_mb / 1024)} GB${g.vulkan_ok ? "" : " (no Vulkan 1.3)"}`)));
      sel.addEventListener("change", () => write(Number(sel.value)));
      ctl = h("div", { class: "select" }, sel);
      break;
    }
    case "number":
    case "range": {
      const out = h("span", { class: "val" }, `${val}${f.suffix ? " " + f.suffix : ""}`);
      const step = (d) => {
        const cur = get(activeEmu(), f.key);
        const nv = Math.max(f.min, Math.min(f.max, cur + d * f.step));
        out.textContent = `${nv}${f.suffix ? " " + f.suffix : ""}`;
        write(nv);
      };
      ctl = h("div", { class: "stepper" },
        h("button", { "data-nav": true, onclick: () => step(-1), title: "Less" }, "−"), out,
        h("button", { "data-nav": true, onclick: () => step(1), title: "More" }, "+"));
      break;
    }
    case "text": {
      const inp = h("input", { class: "text", value: val ?? "", "data-nav": true, spellcheck: "false", style: { width: "220px" } });
      inp.addEventListener("change", () => write(inp.value));
      ctl = inp;
      break;
    }
    case "folder": {
      const inp = h("input", { class: "text", value: val ?? "", placeholder: "Not set", "data-nav": true, spellcheck: "false" });
      inp.addEventListener("change", () => write(inp.value));
      ctl = h("div", { style: { display: "flex", gap: "8px" } }, inp,
        h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("pick_folder"); if (p) { inp.value = p; write(p); } } }, "Browse"));
      break;
    }
    case "color": {
      const inp = h("input", { type: "color", value: val || "#5163f5", "data-nav": true, style: { width: "44px", height: "34px", border: "0", background: "none" } });
      inp.addEventListener("change", () => write(inp.value));
      ctl = h("div", { style: { display: "flex", gap: "8px", alignItems: "center" } }, inp,
        h("button", { class: "btn small", "data-nav": true, onclick: () => { write(""); renderPane(); } }, val ? "Let game decide" : "Game decides"));
      break;
    }
  }
  return h("div", { class: "field" + (locked || requiresOff ? " disabled" : "") },
    h("div", { class: "name" }, f.label, f.tag ? h("span", { class: "tag " + f.tag }, TAG_TEXT[f.tag]) : null),
    f.help ? h("div", { class: "help" }, f.help) : null,
    h("div", { class: "ctl" }, ctl));
}

function paneProfile(pane) {
  const p = S.profile, sys = S.sys;
  if (!p || !sys) { pane.append(h("p", { class: "lead" }, "Reading hardware…")); return; }
  const gpu = sys.gpus.find((g) => g.name === p.gpu_name);
  pane.append(h("div", { class: "pcard" },
    h("div", { class: "micro" }, "Recommended for this PC"),
    h("div", { class: "tier" }, p.label),
    h("div", { class: "hw" },
      h("div", {}, h("div", { class: "micro" }, "Graphics"), h("div", { class: "v" }, gpu ? `${shortGpu(gpu.name)} · ${(gpu.vram_mb / 1024).toFixed(0)} GB` : "None"), gpu ? h("div", { class: "micro", style: { marginTop: "3px" } }, "Vulkan " + gpu.vulkan) : null),
      h("div", {}, h("div", { class: "micro" }, "Processor"), h("div", { class: "v" }, sys.cpu || "—"), h("div", { class: "micro", style: { marginTop: "3px" } }, `${sys.cores} cores · ${sys.threads} threads`)),
      h("div", {}, h("div", { class: "micro" }, "Memory"), h("div", { class: "v" }, Math.round(sys.ram_mb / 1024) + " GB"), h("div", { class: "micro", style: { marginTop: "3px" } }, sys.os))),
    h("ul", {}, p.reasons.map((r) => h("li", {}, r))),
    h("div", { style: { display: "flex", gap: "10px", alignItems: "center" } },
      h("button", { class: "btn primary", "data-nav": true, disabled: p.tier === "unsupported", onclick: applyProfile }, S.settings.auto_profile ? "Apply again" : "Apply these settings"),
      h("button", { class: "btn", "data-nav": true, onclick: async () => { S.sys = await invoke("system_info", { refresh: true }); S.profile = await invoke("auto_profile"); renderPane(); renderTop(); } }, "Detect again"),
      S.settings.auto_profile ? h("span", { class: "micro" }, "Applied: " + S.settings.auto_profile) : null)));
  pane.append(h("div", { class: "micro", style: { margin: "6px 0 10px" } }, "All graphics cards"));
  for (const g of sys.gpus) {
    pane.append(h("div", { class: "field" },
      h("div", { class: "name" }, `#${g.index}  ${g.name}`, h("span", { class: "tag " + (g.vulkan_ok ? "fps-up" : "risk") }, g.vulkan_ok ? "Vulkan " + g.vulkan : "Too old")),
      h("div", { class: "help" }, `${g.kind} · ${g.vram_mb} MB · ${g.vendor}${g.driver ? " · driver " + g.driver : ""}`)));
  }
  if (sys.vulkan_error) pane.append(h("div", { class: "banner" }, sys.vulkan_error));
}

async function applyProfile() {
  try {
    S.settings = await invoke("apply_auto_profile");
    toast(`Applied ${S.profile.label}.`, "ok");
    renderPane(); renderTop();
  } catch (e) { toast(String(e), "err"); }
}

function paneRam(pane) {
  const target = activeEmu();
  const locked = S.perGame && !target._custom;
  const perf = target.perf;
  const total = Math.max(4, Math.round((S.sys?.ram_mb || 16384) / 1024));
  const pct = Math.min(100, (perf.ram_limit_gb / total) * 100);
  const writePerf = (k, v) => {
    if (S.perGame) entry(S.perGame).perf[k] = v; else S.settings.perf[k] = v;
    save(); renderPane();
  };
  const step = (d) => writePerf("ram_limit_gb", Math.max(2, Math.min(total, perf.ram_limit_gb + d)));
  const modeSel = h("select", { "data-nav": true },
    h("option", { value: "soft", selected: perf.ram_mode === "soft" }, "Soft — page out the rest"),
    h("option", { value: "strict", selected: perf.ram_mode === "strict" }, "Strict — close if exceeded"));
  modeSel.addEventListener("change", () => writePerf("ram_mode", modeSel.value));
  const on = perf.ram_limit_enabled;
  pane.append(
    h("div", { class: "field" + (locked ? " disabled" : "") },
      h("div", { class: "name" }, "Limit game memory"),
      h("div", { class: "help" }, "Caps how much RAM the game may use so the rest of your PC stays responsive. Most games want 8 GB or more."),
      h("div", { class: "ctl" }, h("button", { class: "switch" + (on ? " on" : ""), "data-nav": true, onclick: () => writePerf("ram_limit_enabled", !on) }))),
    h("div", { class: "field" + (locked || !on ? " disabled" : "") },
      h("div", { class: "name" }, "Memory for the game"),
      h("div", { class: "help" }, `Your PC has ${total} GB. Suggested: ${S.profile?.ram_limit_gb ?? 8} GB.`),
      h("div", { class: "ctl" },
        h("div", { class: "stepper" },
          h("button", { "data-nav": true, onclick: () => step(-1) }, "−"),
          h("span", { class: "val" }, perf.ram_limit_gb + " GB"),
          h("button", { "data-nav": true, onclick: () => step(1) }, "+"))),
      h("div", { class: "meter" },
        h("div", { class: "track" }, h("div", { class: "fill", style: { width: pct + "%" } }), h("div", { class: "os", style: { width: Math.max(0, 100 - pct) + "%" } })),
        h("div", { class: "legend" }, h("span", { class: "micro" }, `Game ${perf.ram_limit_gb} GB`), h("span", { class: "micro" }, `Windows & apps ${Math.max(0, total - perf.ram_limit_gb)} GB`)))),
    h("div", { class: "field" + (locked || !on ? " disabled" : "") },
      h("div", { class: "name" }, "Limit mode", perf.ram_mode === "strict" ? h("span", { class: "tag risk" }, "May close game") : null),
      h("div", { class: "help" }, "Soft keeps physical RAM under the limit and moves the rest to the page file: slower, but the game keeps running. Strict stops the game if it asks for more."),
      h("div", { class: "ctl" }, h("div", { class: "select" }, modeSel))));
}

async function addFolder() {
  const p = await invoke("pick_folder");
  if (!p) return;
  if (!S.settings.library_folders.includes(p)) S.settings.library_folders.push(p);
  await save(true);
  await refresh();
  if (S.view === "settings") renderPane();
  toast("Folder added.", "ok");
}

function paneLibrary(pane) {
  for (const f of S.settings.library_folders) {
    const isDefault = f === S.boot.games_dir;
    pane.append(h("div", { class: "field" },
      h("div", { class: "name mono", style: { fontSize: "13px", wordBreak: "break-all" } }, f, isDefault ? h("span", { class: "tag debug" }, "Default") : null),
      h("div", { class: "help" }, `${S.games.filter((g) => g.folder === f).length} game(s)`),
      h("div", { class: "ctl" },
        h("button", { class: "btn small", "data-nav": true, onclick: () => invoke("open_path", { path: f }).catch((e) => toast(String(e), "err")) }, "Open"),
        h("button", { class: "btn small danger", "data-nav": true, onclick: async () => {
          S.settings.library_folders = S.settings.library_folders.filter((x) => x !== f);
          await save(true); await refresh(); renderPane();
        } }, "Remove"))));
  }
  pane.append(h("div", { style: { display: "flex", gap: "10px", marginTop: "22px" } },
    h("button", { class: "btn primary", "data-nav": true, onclick: addFolder }, "Add folder"),
    h("button", { class: "btn", "data-nav": true, onclick: async () => { await refresh(); renderPane(); toast(`${S.games.length} games found.`, "ok"); } }, "Scan again")));
}

function paneEngine(pane) {
  const en = S.engine;
  const dl = S.download;
  const pct = dl && dl.total ? Math.round((dl.received / dl.total) * 100) : 0;
  pane.append(
    h("div", { class: "banner " + (en.path ? "" : "accent") },
      h("span", {}, en.path ? (en.version || en.error || "Installed") : "The engine is not installed yet. Strata cannot start games without it."),
      h("span", { class: "chip " + (en.path && !en.error ? "ok" : "warn") }, h("span", { class: "dot" }), en.path ? (en.error ? "Problem" : "Ready") : "Missing")),
    h("div", { class: "field" },
      h("div", { class: "name" }, "Engine location"),
      h("div", { class: "help mono", style: { wordBreak: "break-all" } }, en.path || "—"),
      h("div", { class: "ctl" },
        h("button", { class: "btn small", "data-nav": true, onclick: async () => {
          const p = await invoke("pick_file");
          if (p) { S.settings.engine_path = p; await save(true); await loadEngine(); renderPane(); renderTop(); }
        } }, "Choose file"),
        h("button", { class: "btn small", "data-nav": true, onclick: async () => {
          const p = await invoke("pick_folder");
          if (p) { S.settings.engine_path = p; await save(true); await loadEngine(); renderPane(); renderTop(); }
        } }, "Choose folder"))),
    h("div", { class: "field" },
      h("div", { class: "name" }, "Download the latest engine"),
      h("div", { class: "help" }, "Gets the newest official Windows build from github.com/KytyPS5/KytyPS5 into Strata's data folder (about 100 MB)."),
      h("div", { class: "ctl" }, h("button", { class: "btn small", "data-nav": true, disabled: dl && dl.stage !== "done" && dl.stage !== "error", onclick: downloadEngine }, en.path ? "Update" : "Download")),
      dl ? h("div", { class: "meter" }, h("div", { class: "progress" }, h("div", { style: { width: (dl.stage === "done" ? 100 : pct) + "%" } })),
        h("div", { class: "legend" }, h("span", { class: "micro" }, dl.stage === "error" ? "Error: " + dl.message : dl.stage), h("span", { class: "micro" }, dl.total ? `${(dl.received / 1048576).toFixed(0)} / ${(dl.total / 1048576).toFixed(0)} MB` : ""))) : null),
    h("div", { class: "field" },
      h("div", { class: "name" }, "Use automatic location"),
      h("div", { class: "help" }, "Forget the chosen path and look in Strata's engine folder."),
      h("div", { class: "ctl" },
        h("button", { class: "btn small", "data-nav": true, onclick: async () => { S.settings.engine_path = ""; await save(true); await loadEngine(); renderPane(); } }, "Reset"),
        h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.engine }).catch(() => toast("The engine folder does not exist yet.", "warn")); } }, "Open folder"))));
}

async function downloadEngine() {
  S.download = { stage: "fetching", received: 0, total: 0, message: "" };
  renderPane();
  try {
    const exe = await invoke("download_engine");
    S.settings.engine_path = exe;
    await loadEngine();
    toast("Engine installed.", "ok");
  } catch (e) { toast(String(e), "err"); }
  if (S.view === "settings") renderPane();
  renderTop();
  if ($("#wizard").classList.contains("show")) renderWizard();
}

function paneAppearance(pane) {
  const row = (key, label) => h("div", { class: "field" },
    h("div", { class: "name" }, label),
    h("div", { class: "help" }, key === "accent1" ? "Main buttons, progress and highlights." : "Focus rings and secondary highlights."),
    h("div", { class: "ctl" }, h("div", { class: "swatches" }, ACCENTS.map((a) =>
      h("button", { class: "sw" + (S.settings.ui[key] === a ? " on" : ""), "data-nav": true, title: a, style: { background: PALETTE[a] }, onclick: () => { S.settings.ui[key] = a; applyTheme(); save(); renderPane(); renderTop(); } })))));
  pane.append(row("accent1", "Accent 1"), row("accent2", "Accent 2"),
    h("div", { class: "field" },
      h("div", { class: "name" }, "Preview the boot"),
      h("div", { class: "help" }, "Plays the startup animation with the current colors."),
      h("div", { class: "ctl" }, h("button", { class: "btn small", "data-nav": true, onclick: previewBoot }, "Play"))));
}

function previewBoot() {
  if ($("#boot")) return;
  document.body.append(h("div", { class: "boot", id: "boot" }, h("canvas"), h("div", { class: "micro skip" }, "Press any key to skip")));
  const u = S.settings.ui;
  playBoot({ style: u.boot_style === "off" ? "duo" : u.boot_style, accent1: u.accent1, accent2: u.accent2, version: S.boot.version, reduceMotion: u.reduce_motion });
}

function paneAbout(pane) {
  pane.append(
    h("div", { class: "pcard" },
      h("div", { style: { display: "flex", gap: "18px", alignItems: "center" } }, h("span", { html: MARK(), style: { width: "54px", height: "54px", display: "block" } }),
        h("div", {}, h("div", { class: "tier", style: { margin: 0 } }, "Strata"), h("div", { class: "micro" }, `Version ${S.boot.version} · ${S.boot.platform}`))),
      h("ul", { style: { marginTop: "20px" } },
        h("li", {}, "Game engine: KytyPS5 — GPL-2.0, by its contributors (github.com/KytyPS5/KytyPS5). Based on Kyty (MIT)."),
        h("li", {}, "Strata launcher — GPL-2.0. Fonts: Archivo and IBM Plex Mono (SIL Open Font License)."),
        h("li", {}, "Strata is not affiliated with, endorsed by or connected to any console maker. It ships no games, firmware or keys."),
        h("li", {}, "Only play games you dumped yourself from hardware you own.")),
      h("div", { style: { display: "flex", gap: "10px" } },
        h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.data }); } }, "Open data folder"),
        h("button", { class: "btn small", "data-nav": true, onclick: () => { S.settings.first_run = true; save(); renderWizard(true); } }, "Run setup again"))));
}

// ───────────────────────── developer ─────────────────────────
const logBuf = [];
function renderDev() {
  const root = $("#view-dev .page");
  const f = S.devFilter;
  const lvBtn = (lv) => h("button", { class: f[lv] ? "on" : "", "data-nav": true, onclick: (ev) => { f[lv] = !f[lv]; ev.currentTarget.classList.toggle("on", f[lv]); redrawLog(); } }, lv);
  const srcSel = h("select", { "data-nav": true }, ["all", "app", "engine", "ui", "selftest", "perf"].map((s) => h("option", { value: s, selected: f.source === s }, s === "all" ? "All sources" : s)));
  srcSel.addEventListener("change", () => { f.source = srcSel.value; redrawLog(); });
  const q = h("input", { class: "search", placeholder: "Filter text", value: f.q, "data-nav": true, spellcheck: "false" });
  q.addEventListener("input", () => { f.q = q.value.toLowerCase(); redrawLog(); });
  const follow = h("button", { class: "btn small" + (f.follow ? "" : " ghost"), "data-nav": true, onclick: (ev) => { f.follow = !f.follow; ev.currentTarget.textContent = f.follow ? "Following" : "Paused"; } }, f.follow ? "Following" : "Paused");

  const r = S.running;
  root.replaceChildren(
    h("div", { class: "page-head" },
      h("div", {}, h("div", { class: "micro" }, "Logs · tests · diagnostics"), h("h2", {}, "Developer")),
      h("div", { class: "tools" },
        h("button", { class: "btn small", "data-nav": true, onclick: runTests }, "Run self-test"),
        h("button", { class: "btn small", "data-nav": true, onclick: async () => { try { const p = await invoke("export_diagnostics"); toast("Saved " + p, "ok"); } catch (e) { toast(String(e), "err"); } } }, "Export diagnostics"),
        h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.logs }); } }, "Open logs folder"))),
    h("div", { class: "dev" },
      h("div", { class: "console" },
        h("div", { class: "bar" }, h("div", { class: "filters" }, ["info", "warn", "error", "debug"].map(lvBtn)), h("div", { class: "select" }, srcSel), q, h("span", { style: { flex: 1 } }), follow,
          h("button", { class: "btn small", "data-nav": true, onclick: copyLog }, "Copy"),
          h("button", { class: "btn small", "data-nav": true, onclick: () => { invoke("clear_logs"); logBuf.length = 0; redrawLog(); } }, "Clear")),
        h("div", { class: "lines", id: "loglines" })),
      h("div", { class: "side" },
        h("div", { class: "box" }, h("h4", {}, "Self-test", h("span", { class: "micro" }, S.checks ? `${S.checks.filter((c) => c.status === "pass").length}/${S.checks.length} pass` : "")),
          S.checks ? S.checks.map((c) => h("div", { class: "check " + c.status }, h("span", { class: "st" }), h("div", {}, h("div", {}, c.name), h("div", { class: "d" }, c.detail))))
            : h("div", { class: "micro" }, "Press T or Run self-test.")),
        h("div", { class: "box" }, h("h4", {}, "Session"),
          r ? [h("div", { class: "check pass" }, h("span", { class: "st" }), h("div", {}, h("div", {}, r.title), h("div", { class: "d" }, `PID ${r.pid}`))),
            ...r.notes.map((n) => h("div", { class: "micro", style: { margin: "6px 0", textTransform: "none", letterSpacing: "0" } }, n)),
            h("div", { class: "cmd" }, r.command)]
            : h("div", { class: "micro" }, "No game running.")),
        h("div", { class: "box" }, h("h4", {}, "System"),
          S.sys ? h("div", { class: "cmd" }, `${S.sys.os}\n${S.sys.cpu}\n${Math.round(S.sys.ram_mb / 1024)} GB RAM\n` + S.sys.gpus.map((g) => `#${g.index} ${g.name} · ${g.vram_mb} MB · Vulkan ${g.vulkan}`).join("\n") + `\nEngine: ${S.engine.version || S.engine.error || "—"}\nData: ${S.boot.data_dir}`) : null))));
  $("#loglines").style.whiteSpace = "normal";
  redrawLog();
}

function lineMatches(l) {
  const f = S.devFilter;
  const lv = l.level === "trace" ? "debug" : l.level;
  if (!f[lv]) return false;
  if (f.source !== "all" && l.source !== f.source) return false;
  if (f.q && !(l.msg.toLowerCase().includes(f.q) || l.source.includes(f.q))) return false;
  return true;
}
function lineEl(l) {
  return h("div", { class: "ln " + l.level }, h("span", { class: "tm" }, l.t), h("span", { class: "lv" }, l.level), h("span", { class: "src" }, l.source), h("span", {}, l.msg));
}
function redrawLog() {
  const box = $("#loglines");
  if (!box) return;
  const items = [...logBuf].sort((a, b) => a.seq - b.seq).filter(lineMatches).slice(-1500).map(lineEl);
  box.replaceChildren(...items);
  if (S.devFilter.follow) box.scrollTop = box.scrollHeight;
}
const seenSeq = new Set();
function appendLog(l) {
  if (seenSeq.has(l.seq)) return;
  seenSeq.add(l.seq);
  logBuf.push(l);
  if (logBuf.length > 4000) logBuf.splice(0, logBuf.length - 4000);
  S.logSeq = Math.max(S.logSeq, l.seq);
  if (S.view !== "dev") return;
  const box = $("#loglines");
  if (!box || !lineMatches(l)) return;
  box.append(lineEl(l));
  if (box.childElementCount > 1600) box.firstElementChild.remove();
  if (S.devFilter.follow) box.scrollTop = box.scrollHeight;
}
function copyLog() {
  const text = logBuf.filter(lineMatches).map((l) => `${l.t} ${l.level.padEnd(5)} [${l.source}] ${l.msg}`).join("\n");
  navigator.clipboard.writeText(text).then(() => toast("Log copied.", "ok"), () => toast("Clipboard not available.", "warn"));
}
async function runTests() {
  toast("Running self-test…");
  try {
    S.checks = await invoke("run_selftest");
    const bad = S.checks.filter((c) => c.status === "fail").length;
    toast(bad ? `${bad} check(s) failed.` : "All checks passed.", bad ? "err" : "ok");
  } catch (e) { toast(String(e), "err"); }
  if (S.view === "dev") renderDev();
}

// ───────────────────────── running a game ─────────────────────────
async function launch(id) {
  if (S.running) return;
  if (!S.engine.path) {
    toast("Install the engine first (Settings → Engine).", "warn");
    S.section = "engine"; S.perGame = null; go("settings");
    return;
  }
  try {
    S.running = await invoke("launch_game", { id });
    const e = entry(id);
    e.launches = (e.launches || 0) + 1;
    e.last_played = Math.floor(Date.now() / 1000);
    renderRunning();
    if (S.view === "home") { S.sel = 0; renderHome(); }
  } catch (err) {
    toast(String(err), "err");
  }
}
async function stopGame() {
  try { await invoke("stop_game"); } catch (e) { toast(String(e), "err"); }
}
function renderRunning() {
  const el = $("#running");
  if (!S.running) { el.classList.add("hidden"); return; }
  el.classList.remove("hidden");
  el.replaceChildren(h("span", { class: "pulse" }), h("span", { class: "t" }, S.running.title), h("span", { class: "time", id: "runtime" }, "0:00"),
    h("button", { class: "btn small danger", "data-nav": true, onclick: stopGame }, "Stop"));
}
setInterval(() => {
  const t = $("#runtime");
  if (t && S.running) {
    const s = Math.floor((Date.now() - S.running.started_ms) / 1000);
    t.textContent = `${Math.floor(s / 3600) ? Math.floor(s / 3600) + ":" : ""}${String(Math.floor((s % 3600) / 60)).padStart(Math.floor(s / 3600) ? 2 : 1, "0")}:${String(s % 60).padStart(2, "0")}`;
  }
  const c = $("#clock");
  if (c) c.textContent = clock();
}, 1000);

function toggleFav(id) {
  const e = entry(id);
  e.favorite = !e.favorite;
  save();
  if (S.view === "home") renderHome();
}

// ───────────────────────── first-run wizard ─────────────────────────
function renderWizard(force = false) {
  const m = $("#wizard");
  if (!force && !S.settings.first_run) { m.classList.remove("show"); return; }
  const p = S.profile;
  const step = (n, done, title, desc, action) => h("div", { class: "step" + (done ? " done" : "") },
    h("span", { class: "n" }, done ? "✓" : n), h("div", {}, h("div", { class: "t" }, title), h("div", { class: "d" }, desc)), action);
  const dl = S.download;
  m.replaceChildren(h("div", { class: "sheet" },
    h("div", { class: "micro" }, "Welcome"),
    h("h2", {}, "Let's set up Strata."),
    h("p", {}, "Three quick steps. Everything can be changed later in Settings."),
    h("div", { class: "steps" },
      step(1, !!S.settings.auto_profile, p ? `Your PC: ${p.label}` : "Checking your PC…",
        !p ? "" : p.tier === "unsupported" ? (p.reasons[0] || "Update your graphics driver.") : `${p.gpu_name ? shortGpu(p.gpu_name) + " · " : ""}${Math.round((S.sys?.ram_mb || 0) / 1024)} GB RAM — tuned settings ready.`,
        h("button", { class: "btn small", "data-nav": true, disabled: !p || p.tier === "unsupported", onclick: async () => { await applyProfile(); renderWizard(true); } }, S.settings.auto_profile ? "Applied" : "Apply")),
      step(2, !!S.engine.path, "Game engine", S.engine.path ? (S.engine.version || "Installed") : dl ? `${dl.stage}${dl.total ? ` · ${Math.round((dl.received / dl.total) * 100)}%` : ""}` : "Download the open-source KytyPS5 engine (official GitHub release).",
        S.engine.path ? h("span", { class: "micro" }, "Ready") : h("button", { class: "btn small", "data-nav": true, disabled: dl && dl.stage !== "error" && dl.stage !== "done", onclick: downloadEngine }, "Download")),
      step(3, S.games.length > 0, "Your games", S.games.length ? `${S.games.length} found.` : "Put game folders in the Games folder, or add your own folder.",
        h("div", { style: { display: "flex", gap: "8px" } },
          h("button", { class: "btn small", "data-nav": true, onclick: () => invoke("open_path", { path: S.boot.games_dir }) }, "Open"),
          h("button", { class: "btn small", "data-nav": true, onclick: async () => { await addFolder(); renderWizard(true); } }, "Add folder")))),
    h("div", { style: { display: "flex", justifyContent: "space-between", alignItems: "center" } },
      h("span", { class: "micro" }, "Only use games you own."),
      h("button", { class: "btn primary", "data-nav": true, onclick: () => { S.settings.first_run = false; save(true); m.classList.remove("show"); nav.setScope(null); go("home"); } }, "Start"))));
  m.classList.add("show");
  nav.setScope(m.querySelector(".sheet"));
  setTimeout(() => nav.focus(m.querySelector("[data-nav]:not([disabled])"), { silent: true }), 80);
}

// ───────────────────────── data loading ─────────────────────────
async function refresh() {
  try {
    S.games = await invoke("scan_games");
  } catch (e) { toast(String(e), "err"); }
  if (S.view === "home") renderHome();
  if (S.view === "library") renderLibrary();
}
async function loadEngine() {
  try { S.engine = await invoke("engine_info"); } catch (e) { S.engine = { path: null, version: null, error: String(e) }; }
}

// ───────────────────────── global keys ─────────────────────────
nav.on("back", () => {
  if (closeDrawer()) return true;
  if ($("#wizard").classList.contains("show")) return false;
  if (S.view === "settings" && S.perGame) { S.perGame = null; renderSettings(); return true; }
  if (S.view !== "home") { go("home"); return true; }
  return false;
});
nav.on("tabPrev", () => { if (drawerFor || $("#wizard").classList.contains("show")) return; const v = visibleViews(); go(v[(v.indexOf(S.view) - 1 + v.length) % v.length]); nav.blip("move"); });
nav.on("tabNext", () => { if (drawerFor || $("#wizard").classList.contains("show")) return; const v = visibleViews(); go(v[(v.indexOf(S.view) + 1) % v.length]); nav.blip("move"); });
nav.on("menu", () => { S.perGame = null; go(S.view === "settings" ? "home" : "settings"); });
nav.on("action", (e) => {
  const k = (e.key || "").toLowerCase();
  if (k === "f11") { win.fullscreen(); return true; }
  if (k === "f5") { refresh(); return true; }
  const games = sortedGames();
  if (S.view === "home" && games.length && !drawerFor) {
    const g = games[S.sel];
    if (k === "i" || k === "pad-y") openDrawer(g.id);
    else if (k === "f" || k === "pad-x") toggleFav(g.id);
    else if (k === "s") openGameSettings(g.id);
  } else if (S.view === "library" && k === "/") {
    e.preventDefault?.(); $("#search")?.focus();
  } else if (S.view === "dev") {
    if (k === "t") runTests();
    if (k === "c") { invoke("clear_logs"); logBuf.length = 0; redrawLog(); }
  }
});
document.addEventListener("padchange", () => renderTop());
$("#scrim").addEventListener("click", closeDrawer);
window.addEventListener("error", (e) => log("error", `UI error: ${e.message} @ ${e.filename}:${e.lineno}`));
window.addEventListener("unhandledrejection", (e) => log("error", `UI promise rejected: ${e.reason}`));

// ───────────────────────── start ─────────────────────────
(async function main() {
  S.boot = await invoke("bootstrap");
  S.settings = S.boot.settings;
  S.running = S.boot.running;
  applyTheme();
  const u = S.settings.ui;
  const bootDone = playBoot({ style: u.boot_style, accent1: u.accent1, accent2: u.accent2, version: S.boot.version, reduceMotion: u.reduce_motion });

  // load everything while the boot plays
  listen("log", appendLog);
  listen("game-start", (r) => { S.running = r; renderRunning(); });
  listen("game-exit", (x) => {
    S.running = null;
    renderRunning();
    toast(`${S.games.find((g) => g.id === x.game_id)?.title || "Game"} — ${x.message}`, x.code && x.code !== 0 ? "err" : "ok");
    if (S.settings.games[x.game_id]) S.settings.games[x.game_id].play_seconds += x.seconds;
    if (S.view === "home") renderHome();
    if (S.view === "dev") renderDev();
  });
  listen("engine-download", (p) => {
    S.download = p;
    if (S.view === "settings" && S.section === "engine") renderPane();
    if ($("#wizard").classList.contains("show")) renderWizard(true);
  });
  const [games] = await Promise.all([
    invoke("scan_games").catch(() => []),
    invoke("system_info", { refresh: false }).then((s) => (S.sys = s)).catch(() => null),
    loadEngine(),
    invoke("logs_since", { seq: 0 }).then((ls) => ls.forEach(appendLog)).catch(() => null),
  ]);
  S.games = games;
  S.profile = await invoke("auto_profile").catch(() => null);
  log("info", `UI ready · ${S.games.length} games · engine ${S.engine.path ? "found" : "missing"}`);
  await bootDone;
  renderRunning();
  go("home");
  if (S.settings.first_run) renderWizard();
})();
