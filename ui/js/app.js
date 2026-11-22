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
// Native append() would print "null" for optional pieces; skip them.
const add = (el, ...kids) => el.append(...kids.flat().filter((k) => k !== null && k !== undefined && k !== false));
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
  downloads: {}, // engine id -> progress
  engines: [],
  controlInfo: null,
  backups: null,
  skipped: [],
  monitorHz: null,
  pkgs: [], // loose .pkg files found in the library folders
  pkgJob: null, // install progress
  extras: {}, // game id -> update/DLC/mods
  states: {}, // game id -> { state: worked|failed|untested, engine }
  stats: null,
  sizes: {},
};
const ENGINE_NAME = { kyty: "KytyPS5", sharpemu: "SharpEmu", anyps5: "AnyPS5", shadps4: "shadPS4" };
const PS5_SET = ["kyty", "sharpemu", "anyps5"];
// Support links. Fill these in to show the "Support Strata" buttons (About page and setup).
const SUPPORT = { kofi: "", sponsors: "", youtube: "" };
const SUPPORT_LABEL = { kofi: "Ko-fi", sponsors: "GitHub Sponsors", youtube: "YouTube" };
const PS4 = () => !!S.boot?.ps4; // PS4 support is held back in this release
const haveEngine = (platform) => S.engines.some((e) => e.path && e.platforms.includes(platform));

const VIEWS = ["home", "library", "settings", "dev"];
const visibleViews = () => VIEWS.filter((v) => v !== "dev" || S.settings?.dev.enabled);

// ───────────────────────── helpers ─────────────────────────
// Re-rendering replaces elements; put focus back on the control at the same position.
function keepFocus(container, render) {
  const box = typeof container === "string" ? $(container) : container;
  const had = box && box.contains(document.activeElement) && document.activeElement.matches("[data-nav]");
  const idx = had ? [...box.querySelectorAll("[data-nav]")].indexOf(document.activeElement) : -1;
  render();
  if (idx >= 0) {
    const now = typeof container === "string" ? $(container) : container;
    const list = now ? [...now.querySelectorAll("[data-nav]")] : [];
    const el = list[Math.min(idx, list.length - 1)];
    if (el) nav.focus(el, { silent: true });
  }
}
const get = (obj, path) => path.split(".").reduce((o, k) => (o == null ? o : o[k]), obj);
function set(obj, path, val) {
  const ks = path.split(".");
  let o = obj;
  for (const k of ks.slice(0, -1)) o = o[k];
  o[ks.at(-1)] = val;
}
const fmtTime = (secs, launches = 0) => {
  if (!secs) return launches ? "Under a minute" : "Never played";
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
const initials = (t) => (t.match(/[\p{L}\p{N}]+/gu) || ["?"]).slice(0, 2).map((w) => w[0]).join("").toUpperCase();
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
  document.documentElement.classList.toggle("tv", !!u.tv_mode);
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
    h("span", { html: MARK() }), h("span", { class: "word", "data-tauri-drag-region": true }, "STRATA"),
    S.boot.channel ? h("span", { class: "beta" }, S.boot.channel) : null);
  const tabs = h("nav", { class: "tabs" }, visibleViews().map((v, i) =>
    h("button", { class: "tab" + (S.view === v ? " active" : ""), "data-nav": true, id: "tab-" + v, onclick: () => { if (v === "settings") S.perGame = null; go(v); } },
      { home: "Home", library: "Library", settings: "Settings", dev: "Developer" }[v])));
  const gpu = S.profile;
  const chip = h("button", {
    class: "chip " + (!gpu ? "" : gpu.tier === "unsupported" ? "bad" : S.engines.some((e) => e.path) ? "ok" : "warn"),
    "data-nav": true,
    title: "Hardware profile",
    onclick: () => { S.section = "profile"; S.perGame = null; go("settings"); },
  }, h("span", { class: "dot" }), gpu ? (gpu.gpu_name ? shortGpu(gpu.gpu_name) + " · " : "") + gpu.label : S.loaded ? "Hardware unknown" : "Checking PC…");
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
    home: [k("↵", "Play"), k("I", "Details"), k("F", "Favorite"), k("S", "Game settings"), k("F9", "TV mode")],
    library: [k("↵", "Open"), k("/", "Search"), k("F", "Favorite"), k("F9", "TV mode")],
    settings: [k("↵", "Change"), k("←→", "Adjust"), k("Esc", "Back")],
    dev: [k("T", "Run tests"), k("C", "Clear log")],
  }[S.view] || [];
  $("#hints").replaceChildren(...items, h("span", { class: "right" }, h("span", {}, h("span", { class: "k" }, "Q"), h("span", { class: "k" }, "E"), "Switch tab"), k("F11", "Fullscreen")));
}

function go(view) {
  if (!visibleViews().includes(view)) view = "home";
  if (view !== "settings") S.perGame = null; // game settings close when you leave them
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
      h("span", { class: "micro" }, g.archive ? "Archive" : g.title_id || "Folder"),
      h("span", { class: "plat " + g.platform }, g.platform.toUpperCase())),
    g.logo ? h("img", { class: "logo", src: fileSrc(g.logo), alt: g.title }) : h("h1", { class: g.title.length > 22 ? "long" : "" }, g.title),
    h("div", { class: "meta" },
      h("span", {}, "Played ", h("b", {}, fmtTime(e.play_seconds, e.launches))),
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
        xe.favorite ? h("span", { class: "fav" }) : null,
        x.platform === "ps4" ? h("span", { class: "plat ps4 corner" }, "PS4") : null,
        xe.emu ? h("span", { class: "own", title: "Uses its own settings" }) : null),
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
      if (["worked", "failed", "untested"].includes(S.lib.filter)) return (S.states[g.id]?.state || "untested") === S.lib.filter;
      return true;
    })
    .filter((g) => !S.lib.engine || S.states[g.id]?.engine === S.lib.engine)
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

  root.replaceChildren(...[
    h("div", { class: "page-head" },
      h("div", {}, h("div", { class: "micro" }, `${S.games.length} games · ${S.settings.library_folders.length} folders`), h("h2", {}, "Library")),
      h("div", { class: "tools" },
        search,
        seg("filter", [["all", "All"], ["fav", "Favorites"], ["recent", "Recent"], ["worked", "Works"], ["untested", "Untested"], ["failed", "Failed"]]),
        engineFilter(),
        seg("layout", [["grid", "Grid"], ["list", "List"], ["stats", "Stats"]]),
        PS4() ? h("button", { class: "btn small", "data-nav": true, onclick: () => installPkg() }, "Install .pkg") : null,
        h("button", { class: "btn small", "data-nav": true, onclick: refresh }, "Refresh"))),
    PS4() ? pkgBanner() : null,
    h("div", { class: "scroll", id: "libbody" })].filter(Boolean));
  renderLibraryBody(list);
}

function engineFilter() {
  const sel = h("select", { "data-nav": true, title: "Engine that ran the game" },
    h("option", { value: "", selected: !S.lib.engine }, "Any engine"),
    S.engines.filter((e) => e.platforms.includes("ps5") || PS4()).map((e) => h("option", { value: e.id, selected: S.lib.engine === e.id }, e.name)));
  sel.addEventListener("change", () => { S.lib.engine = sel.value; renderLibraryBody(); });
  return h("div", { class: "select small" }, sel);
}

async function loadStates() {
  try { S.states = await invoke("library_states"); } catch { S.states = {}; }
}

function renderStats(body) {
  const st = S.stats;
  if (!st) { body.append(h("div", { class: "micro" }, "Counting…")); invoke("stats").then((x) => { S.stats = x; if (S.lib.layout === "stats") renderLibraryBody(); }); return; }
  const hours = (sec) => sec >= 3600 ? `${(sec / 3600).toFixed(1)} h` : `${Math.round(sec / 60)} min`;
  const tile = (label, value, sub) => h("div", { class: "stat" }, h("div", { class: "micro" }, label), h("div", { class: "big" }, value), sub ? h("div", { class: "micro sub" }, sub) : null);
  const max = Math.max(1, ...st.top.map((t) => t.seconds));
  const engines = Object.entries(st.engines).sort((a, b) => b[1][1] - a[1][1]);
  add(body,
    h("div", { class: "stats-row" },
      tile("Time played", hours(st.total_seconds), `${st.launches} launches`),
      tile("Games", String(st.games), `${st.worked} work · ${st.failed} failed · ${st.untested} untested`),
      tile("Most used engine", engines[0] ? (ENGINE_NAME[engines[0][0]] || engines[0][0]) : "—", engines[0] ? `${engines[0][1][1]} of ${engines[0][1][0]} tests worked` : "Play a game to see it"),
      tile("Made with", Object.keys(st.made_with).length ? Object.entries(st.made_with).sort((a, b) => b[1] - a[1])[0][0] : "—", Object.entries(st.made_with).map(([k, v]) => `${k} ${v}`).join(" · "))),
    h("h4", { class: "sub" }, "Most played"),
    st.top.length ? h("div", { class: "bars" }, st.top.map((t) => h("button", { class: "bar", "data-nav": true, onclick: () => openDrawer(t.id) },
      h("span", { class: "t" }, t.title), h("span", { class: "track" }, h("span", { style: { width: Math.max(3, (t.seconds / max) * 100) + "%" } })), h("span", { class: "v mono" }, hours(t.seconds)))))
      : h("div", { class: "micro" }, "Nothing played yet."),
    engines.length ? h("h4", { class: "sub" }, "Engines") : null,
    engines.map(([id, [tests, ok]]) => h("div", { class: "field" }, h("div", { class: "name" }, ENGINE_NAME[id] || id), h("div", { class: "help" }, `${ok} of ${tests} tests worked`),
      h("div", { class: "ctl" }, h("span", { class: "micro" }, Math.round((ok / Math.max(1, tests)) * 100) + "%")))));
}

function renderLibraryBody(list = filteredGames()) {
  const body = $("#libbody");
  if (!body) return;
  body.replaceChildren();
  if (S.lib.layout === "stats") return renderStats(body);
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
        h("div", { class: "s micro" }, g.archive ? "ZAR archive" : g.title_id || "—", g.platform === "ps4" ? h("span", { class: "plat ps4" }, "PS4") : null,
          S.states[g.id]?.state === "worked" ? h("span", { class: "plat works", title: "Ran with " + (ENGINE_NAME[S.states[g.id].engine] || "") }, "WORKS") : S.states[g.id]?.state === "failed" ? h("span", { class: "plat failed" }, "FAILED") : null));
    })));
  } else {
    body.append(h("div", { class: "list" },
      h("div", { class: "row head" }, h("span"), h("span", {}, "Title"), h("span", {}, "ID"), h("span", {}, "Version"), h("span", {}, "Played"), h("span", {}, "Status")),
      list.map((g) => {
        const e = S.settings.games[g.id] || {};
        return h("button", { class: "row", "data-nav": true, onclick: () => openDrawer(g.id), "data-id": g.id },
          g.icon ? h("img", { src: fileSrc(g.icon), alt: "" }) : h("span", { class: "ph" }, initials(g.title)),
          h("span", { class: "t" }, g.title), h("span", { class: "mono" }, g.title_id || "—"), h("span", { class: "mono" }, g.version || "—"),
          h("span", {}, fmtTime(e.play_seconds, e.launches)), h("span", {}, STATUS[e.status] || "Untested"));
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
        h("div", {}, h("div", { class: "micro" }, "Time played"), h("div", { class: "v" }, fmtTime(e.play_seconds, e.launches))),
        h("div", {}, h("div", { class: "micro" }, "Launches"), h("div", { class: "v" }, String(e.launches || 0))),
        h("div", {}, h("div", { class: "micro" }, "Size"), sizeEl),
        h("div", {}, h("div", { class: "micro" }, "Last played"), h("div", { class: "v" }, fmtAgo(e.last_played))),
        h("div", {}, h("div", { class: "micro" }, "Platform"), h("div", { class: "v" }, g.platform.toUpperCase())),
        h("div", {}, h("div", { class: "micro" }, "Engine"), h("div", { class: "v" }, e.engine ? ENGINE_NAME[e.engine] : e.good_engine ? `Auto · last worked: ${ENGINE_NAME[e.good_engine]}` : "Auto")),
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

// ───────────────────────── PS4 .pkg install ─────────────────────────
function pkgBanner() {
  const job = S.pkgJob;
  if (job && job.stage === "extracting") {
    const pct = job.bytes_total ? Math.min(100, Math.round((job.bytes_done / job.bytes_total) * 100)) : 0;
    return h("div", { class: "banner accent pkgbar", id: "pkgbar" },
      h("span", {}, `Installing ${job.file} · ${job.files_done}${job.files_total ? " / " + job.files_total : ""} files · ${pct}%`),
      h("div", { class: "progress" }, h("div", { style: { width: pct + "%" } })));
  }
  if (!S.pkgs.length) return null;
  return h("div", { class: "banner", id: "pkgbar" },
    h("span", {}, S.pkgs.length === 1 ? `Found a .pkg in your folders: ${S.pkgs[0].split(/[\\/]/).pop()}` : `Found ${S.pkgs.length} .pkg files in your folders.`),
    h("button", { class: "btn small", "data-nav": true, onclick: () => installPkg(S.pkgs[0]) }, S.pkgs.length === 1 ? "Install" : "Install the first"));
}

async function installPkg(path, overwrite = false) {
  if (S.pkgJob && S.pkgJob.stage === "extracting") return toast("Another package is installing.", "warn");
  if (!path) path = await invoke("pick_pkg");
  if (!path) return;
  let plan;
  let folder = null;
  for (;;) {
    try { plan = await invoke("pkg_inspect", { path, folder }); } catch (e) { return toast(String(e), "err"); }
    if (overwrite) break;
    const i = plan.info;
    const where = i.kind === "update" ? (plan.game_found ? "next to the installed game (as an update)" : "in your games folder (the game itself is not installed yet)")
      : i.kind === "dlc" ? "in the PS4 engine's DLC folder" : "in your games folder";
    const answer = await ask(`Install ${i.kind_name}: ${i.title_id}?`,
      `${i.content_id} · ${fmtSize(i.size_mb)}. It goes ${where}:\n${plan.destination}${plan.exists ? "\n\nIt is already installed and will be replaced." : ""}`,
      plan.exists ? "Replace" : "Install", "Cancel", i.kind === "game" ? "Change folder" : null);
    if (answer === "extra") {
      const p = await invoke("pick_folder");
      if (p) {
        folder = p;
        S.settings.pkg_install_folder = p;
        if (!S.settings.library_folders.includes(p)) S.settings.library_folders.push(p);
        await save(true);
      }
      continue;
    }
    if (!answer) return;
    break;
  }
  const i = plan.info;
  S.pkgJob = { file: path.split(/[\\/]/).pop(), stage: "extracting", files_done: 0, files_total: 0, bytes_done: 0, bytes_total: 1 };
  if (S.view === "library") renderLibrary();
  try {
    await invoke("pkg_install", { path, folder, overwrite: overwrite || plan.exists });
    toast(`${i.kind_name} ${i.title_id} installed.`, "ok");
    if (i.kind === "game" && !haveEngine("ps4") && !S.settings.engines.skip_ps4) {
      if (await ask("Get the PS4 engine?", "PS4 games run with shadPS4. Download it now?", "Download")) downloadEngine("shadps4");
    }
  } catch (e) {
    toast(String(e).replace(/^already-installed:/, "Already installed: "), "err");
  }
  S.pkgJob = null;
  S.extras = {};
  await refresh();
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
  const sections = SECTIONS.filter((s) => (S.perGame ? s.perGame : s.perGame !== "only") && (!s.devOnly || S.settings.dev.enabled) && (!s.ps4Only || PS4()));
  if (!sections.find((s) => s.id === S.section)) S.section = sections[0].id;

  let lastGroup = null;
  const list = h("div", { class: "sections" });
  if (game) list.append(h("button", { class: "back", "data-nav": true, onclick: () => { S.perGame = null; renderSettings(); } }, h("span", {}, "← Global settings")));
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
  if (!$("#pane") || S.view !== "settings") return;
  keepFocus("#view-settings .page", renderPaneInner);
}
function renderPaneInner() {
  const pane = $("#pane");
  const sec = SECTIONS.find((s) => s.id === S.section);
  pane.replaceChildren(...[h("h3", {}, sec.title), sec.lead ? h("p", { class: "lead" }, sec.lead) : null].filter(Boolean));

  if (S.perGame && sec.fields) {
    const e = entry(S.perGame);
    const on = !!e.emu;
    const diffs = on ? countDiffs(e) : 0;
    add(pane, h("div", { class: "banner " + (on ? "accent" : "") },
      h("span", {}, on ? `This game uses its own settings${diffs ? ` (${diffs} different from global, marked with a dot)` : ""}. Changes here affect only this game.` : "This game follows the global settings. Turn on custom settings to change them for this game only."),
      on ? h("button", { class: "btn small", "data-nav": true, onclick: () => { e.emu = null; e.perf = null; save(); renderPane(); toast("Back to the global settings.", "ok"); } }, "Use global settings again")
        : h("button", { class: "switch", "data-nav": true, onclick: () => { e.emu = structuredClone(S.settings.emu); e.perf = structuredClone(S.settings.perf); save(); renderPane(); } })));
  }

  const custom = { profile: paneProfile, ram: paneRam, library: paneLibrary, engine: paneEngines, appearance: paneAppearance, about: paneAbout, controls: paneControls, backups: paneBackups, gamedev: paneGameDev, extras: paneExtras, compat: paneCompat, community: paneCommunity }[sec.custom];
  if (custom) custom(pane);
  if (sec.fields) {
    const target = activeEmu();
    const locked = S.perGame && !target._custom;
    for (const f of sec.fields) add(pane, field(f, target, locked));
  }
  if (sec.id === "display" || sec.id === "rendering") {
    add(pane, h("div", { style: { marginTop: "26px", display: "flex", gap: "10px" } },
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

function countDiffs(e) {
  let n = 0;
  for (const sec of SECTIONS) for (const f of sec.fields || []) {
    if (!sec.perGame) continue;
    const src = f.key.startsWith("emu.") ? e.emu : f.key.startsWith("perf.") ? e.perf : null;
    if (src && JSON.stringify(src[f.key.slice(f.key.indexOf(".") + 1)]) !== JSON.stringify(get(S.settings, f.key))) n++;
  }
  return n;
}

function field(f, target, locked) {
  const val = get(target, f.key);
  const differs = S.perGame && target._custom && (f.key.startsWith("emu.") || f.key.startsWith("perf.")) && JSON.stringify(val) !== JSON.stringify(get(S.settings, f.key));
  const requiresOff = f.requires && !get(target, f.requires);
  const write = (v) => {
    if (S.perGame && (f.key.startsWith("emu.") || f.key.startsWith("perf."))) {
      const e = entry(S.perGame);
      if (!e.emu) return; // following global settings: nothing to change here
      if (!e.perf) e.perf = structuredClone(S.settings.perf);
      set({ emu: e.emu, perf: e.perf }, f.key, v);
    } else set(S.settings, f.key, v);
    if (f.key.startsWith("ui.")) applyTheme();
    if (f.key === "dev.enabled") renderTop();
    save();
    if (S.perGame || SECTIONS.find((s) => s.id === S.section)?.fields?.some((x) => x.requires === f.key)) renderPane();
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
    case "refresh": {
      const auto = S.monitorHz ? `Auto (${S.monitorHz} Hz on this monitor)` : "Auto (match monitor)";
      const opts = [[0, auto], [30, "30 Hz"], [60, "60 Hz"], [75, "75 Hz"], [90, "90 Hz"], [120, "120 Hz"], [144, "144 Hz"], [165, "165 Hz"], [240, "240 Hz"], [280, "280 Hz"], [360, "360 Hz"]];
      if (!opts.some((o) => o[0] === val)) opts.push([val, val + " Hz"]);
      const sel = h("select", { "data-nav": true }, opts.map(([v, l]) => h("option", { value: String(v), selected: v === val }, l)));
      sel.addEventListener("change", () => write(Number(sel.value)));
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
    h("div", { class: "name" }, differs ? h("span", { class: "diff", title: "Different from the global setting" }) : null, f.label, f.tag ? h("span", { class: "tag " + f.tag }, TAG_TEXT[f.tag]) : null),
    f.help ? h("div", { class: "help" }, f.help) : null,
    h("div", { class: "ctl" }, ctl));
}

function paneProfile(pane) {
  const p = S.profile, sys = S.sys;
  if (!p || !sys) {
    add(pane, h("div", { class: "banner" }, S.loaded ? "Could not read the hardware. Check Developer → Logs." : "Reading hardware…",
      h("button", { class: "btn small", "data-nav": true, onclick: async () => { S.sys = await invoke("system_info", { refresh: true }).catch(() => null); S.profile = await invoke("auto_profile").catch(() => null); renderPane(); renderTop(); } }, "Try again")));
    return;
  }
  const gpu = sys.gpus.find((g) => g.name === p.gpu_name);
  add(pane, h("div", { class: "pcard" },
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
  add(pane, h("div", { class: "micro", style: { margin: "6px 0 10px" } }, "All graphics cards"));
  for (const g of sys.gpus) {
    add(pane, h("div", { class: "field" },
      h("div", { class: "name" }, `#${g.index}  ${g.name}`, h("span", { class: "tag " + (g.vulkan_ok ? "fps-up" : "risk") }, g.vulkan_ok ? "Vulkan " + g.vulkan : "Too old")),
      h("div", { class: "help" }, `${g.kind} · ${g.vram_mb} MB · ${g.vendor}${g.driver ? " · driver " + g.driver : ""}`)));
  }
  if (sys.vulkan_error) add(pane, h("div", { class: "banner" }, sys.vulkan_error));
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
    if (S.perGame) {
      const e = entry(S.perGame);
      if (!e.emu) return;
      if (!e.perf) e.perf = structuredClone(S.settings.perf);
      e.perf[k] = v;
    } else S.settings.perf[k] = v;
    save(); renderPane();
  };
  const step = (d) => writePerf("ram_limit_gb", Math.max(2, Math.min(total, perf.ram_limit_gb + d)));
  const modeSel = h("select", { "data-nav": true },
    h("option", { value: "soft", selected: perf.ram_mode === "soft" }, "Soft — page out the rest"),
    h("option", { value: "strict", selected: perf.ram_mode === "strict" }, "Strict — close if exceeded"));
  modeSel.addEventListener("change", () => writePerf("ram_mode", modeSel.value));
  const on = perf.ram_limit_enabled;
  add(pane, 
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
    add(pane, h("div", { class: "field" },
      h("div", { class: "name mono", style: { fontSize: "13px", wordBreak: "break-all" } }, f, isDefault ? h("span", { class: "tag debug" }, "Default") : null),
      h("div", { class: "help" }, `${S.games.filter((g) => g.folder === f).length} game(s)`),
      h("div", { class: "ctl" },
        h("button", { class: "btn small", "data-nav": true, onclick: () => invoke("open_path", { path: f }).catch((e) => toast(String(e), "err")) }, "Open"),
        h("button", { class: "btn small danger", "data-nav": true, onclick: async () => {
          S.settings.library_folders = S.settings.library_folders.filter((x) => x !== f);
          await save(true); await refresh(); renderPane();
        } }, "Remove"))));
  }
  add(pane, h("div", { style: { display: "flex", gap: "10px", marginTop: "22px" } },
    h("button", { class: "btn primary", "data-nav": true, onclick: addFolder }, "Add folder"),
    h("button", { class: "btn", "data-nav": true, onclick: async () => { await refresh(); renderPane(); toast(`${S.games.length} games found.`, "ok"); } }, "Scan again")));
  if (!PS4()) return;
  const target = h("div", { class: "help mono", style: { wordBreak: "break-all" } }, "…");
  invoke("pkg_target_folder").then((t) => (target.textContent = t)).catch(() => (target.textContent = "—"));
  add(pane, h("h4", { class: "sub" }, "Installing .pkg games"),
    h("div", { class: "field" },
      h("div", { class: "name" }, "Install games to", S.settings.pkg_install_folder ? null : h("span", { class: "tag debug" }, "Automatic")),
      target,
      h("div", { class: "ctl" },
        h("button", { class: "btn small", "data-nav": true, onclick: async () => {
          const p = await invoke("pick_folder");
          if (!p) return;
          S.settings.pkg_install_folder = p;
          if (!S.settings.library_folders.includes(p)) S.settings.library_folders.push(p);
          await save(true); renderPane();
        } }, "Choose folder"),
        S.settings.pkg_install_folder ? h("button", { class: "btn small ghost", "data-nav": true, onclick: async () => { S.settings.pkg_install_folder = ""; await save(true); renderPane(); } }, "Automatic") : null)),
    h("div", { class: "micro", style: { textTransform: "none", letterSpacing: 0 } }, "Automatic uses your own games folder (for example E:\\PS5Games) instead of Strata's folder on drive C."));
}

function dlMeter(dl) {
  if (!dl) return null;
  const pct = dl.total ? Math.round((dl.received / dl.total) * 100) : 0;
  return h("div", { class: "meter" }, h("div", { class: "progress" }, h("div", { style: { width: (dl.stage === "done" ? 100 : pct) + "%" } })),
    h("div", { class: "legend" }, h("span", { class: "micro" }, dl.stage === "error" ? "Error: " + dl.message : dl.stage), h("span", { class: "micro" }, dl.total ? `${(dl.received / 1048576).toFixed(0)} / ${(dl.total / 1048576).toFixed(0)} MB` : "")));
}
const busy = (id) => { const d = S.downloads[id]; return d && d.stage !== "done" && d.stage !== "error"; };

function paneEngines(pane) {
  if (S.perGame) return paneGameEngine(pane);
  const es = S.settings.engines;
  for (const en of S.engines) {
    const dl = S.downloads[en.id];
    add(pane, h("div", { class: "field engine-card" },
      h("div", { class: "name" }, en.name, en.platforms.map((p) => h("span", { class: "plat " + p }, p.toUpperCase())),
        h("span", { class: "chip " + (en.path ? "ok" : "warn"), style: { marginLeft: "8px" } }, h("span", { class: "dot" }), en.path ? "Installed" : "Not installed")),
      h("div", { class: "help" }, en.summary, en.path ? h("div", { class: "mono", style: { fontSize: "11px", marginTop: "4px", wordBreak: "break-all" } }, en.release || en.path) : null),
      h("div", { class: "ctl" },
        h("button", { class: "btn small", "data-nav": true, disabled: busy(en.id), onclick: () => downloadEngine(en.id) }, en.path ? "Update" : "Download"),
        h("button", { class: "btn small", "data-nav": true, onclick: async () => {
          const p = await invoke("pick_folder");
          if (!p) return;
          es.paths[en.id] = p; await save(true); await loadEngines();
          if (!S.engines.find((x) => x.id === en.id)?.path) toast(`No ${en.name} program found in that folder.`, "warn");
          renderPane(); renderTop();
        } }, "Choose folder"),
        es.paths[en.id] ? h("button", { class: "btn small ghost", "data-nav": true, onclick: async () => { delete es.paths[en.id]; await save(true); await loadEngines(); renderPane(); } }, "Reset") : null),
      dlMeter(dl),
      en.id === "shadps4" && en.path ? h("div", { class: "help", style: { marginTop: "8px" } }, "Some PS4 games need system modules (libSceLibcInternal.sprx, libSceNgs2.sprx…) dumped from your own console. Put them in shadPS4's sys_modules folder. ",
        h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.shadps4_modules }).catch(() => toast("Start a PS4 game once to create the folder.", "warn")); } }, "Open sys_modules")) : null));
  }
  const orderRow = (label, key, platform) => {
    const ids = es[key];
    return h("div", { class: "field" },
      h("div", { class: "name" }, label),
      h("div", { class: "help" }, "Strata tries these from left to right. Use the arrows to change the order."),
      h("div", { class: "ctl order" }, ids.map((id, i) => h("span", { class: "pill" + (S.engines.find((e) => e.id === id)?.path ? "" : " off") },
        i > 0 ? h("button", { "data-nav": true, title: "Earlier", onclick: () => { [ids[i - 1], ids[i]] = [ids[i], ids[i - 1]]; save(); renderPane(); } }, "‹") : null,
        ENGINE_NAME[id] || id,
        i < ids.length - 1 ? h("button", { "data-nav": true, title: "Later", onclick: () => { [ids[i + 1], ids[i]] = [ids[i], ids[i + 1]]; save(); renderPane(); } }, "›") : null))));
  };
  add(pane, 
    h("h4", { class: "sub" }, "Automatic engine choice"),
    orderRow("PS5 games", "order_ps5", "ps5"),
    PS4() ? orderRow("PS4 games", "order_ps4", "ps4") : null,
    field({ key: "engines.auto_fallback", type: "toggle", label: "Try the next engine automatically", help: "If an engine closes or crashes right away, Strata starts the next one and writes everything to the log." }, S.settings),
    field({ key: "engines.fail_seconds", type: "number", label: "Counts as failed before", min: 5, max: 120, step: 5, suffix: "s", requires: "engines.auto_fallback", help: "An engine that closes sooner than this is treated as not working for that game." }, S.settings),
    h("div", { class: "field" }, h("div", { class: "name" }, "Engines folder"), h("div", { class: "help" }, "Releases, saves and caches of every engine."),
      h("div", { class: "ctl" }, h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.engines }).catch(() => toast("Nothing downloaded yet.", "warn")); } }, "Open folder"))));
}

function paneGameEngine(pane) {
  const g = S.games.find((x) => x.id === S.perGame);
  const e = entry(S.perGame);
  const fits = S.engines.filter((en) => en.platforms.includes(g.platform) && (!g.archive || en.archives));
  const sel = h("select", { "data-nav": true },
    h("option", { value: "", selected: !e.engine }, "Automatic (recommended)"),
    fits.map((en) => h("option", { value: en.id, selected: e.engine === en.id }, en.name + (en.path ? "" : " — not installed"))));
  sel.addEventListener("change", () => { e.engine = sel.value; save(); renderPane(); });
  add(pane, 
    h("div", { class: "banner" }, h("span", {}, `${g.platform.toUpperCase()} game. ${e.good_engine ? `Last engine that ran it: ${ENGINE_NAME[e.good_engine]}.` : "No engine has run it yet."}`)),
    h("div", { class: "field" }, h("div", { class: "name" }, "Engine for this game"),
      h("div", { class: "help" }, "Automatic tries the engines in order (Settings → Engines) and remembers the one that worked."),
      h("div", { class: "ctl" }, h("div", { class: "select" }, sel))),
    e.good_engine ? h("div", { class: "field" }, h("div", { class: "name" }, "Forget the engine that worked"), h("div", { class: "help" }, "Next launch starts again from the first engine."),
      h("div", { class: "ctl" }, h("button", { class: "btn small", "data-nav": true, onclick: () => { e.good_engine = ""; save(); renderPane(); } }, "Forget"))) : null);
}

async function downloadEngine(id = "kyty") {
  S.downloads[id] = { stage: "fetching", received: 0, total: 0, message: "", engine: id };
  if (id === "kyty") S.download = S.downloads[id];
  if (S.view === "settings") renderPane();
  if ($("#wizard").classList.contains("show")) renderWizard(true);
  let ok = false;
  try {
    const exe = await invoke("download_engine", { id });
    S.settings.engines.paths[id] = exe;
    if (id === "kyty") S.settings.engine_path = exe;
    if (id === "shadps4") S.settings.engines.skip_ps4 = false;
    ok = true;
    toast(`${ENGINE_NAME[id]} installed.`, "ok");
  } catch (e) { toast(`${ENGINE_NAME[id]}: ${e}`, "err"); }
  await loadEngines();
  if (S.view === "settings") renderPane();
  renderTop();
  if ($("#wizard").classList.contains("show")) renderWizard(true);
  return ok;
}
async function downloadMany(ids) {
  for (const id of ids) if (!S.engines.find((e) => e.id === id)?.path) await downloadEngine(id);
}

function paneControls(pane) {
  const info = S.controlInfo;
  if (!info) { invoke("control_info").then((x) => { S.controlInfo = x; renderPane(); }); return; }
  const own = S.perGame ? entry(S.perGame) : null;
  const c = own ? own.controls || S.settings.controls : S.settings.controls;
  const editable = !own || !!own.controls;
  const writeC = (fn) => {
    if (own && !own.controls) return;
    fn(c); save(); renderPane();
  };
  if (own) {
    add(pane, h("div", { class: "banner " + (own.controls ? "accent" : "") },
      h("span", {}, own.controls ? "This game has its own keys." : "This game uses the global keys."),
      own.controls ? h("button", { class: "btn small", "data-nav": true, onclick: () => { own.controls = null; save(); renderPane(); } }, "Use global keys again")
        : h("button", { class: "btn small", "data-nav": true, onclick: () => { own.controls = structuredClone(S.settings.controls); own.controls.enabled = true; save(); renderPane(); } }, "Custom keys for this game")));
  }
  add(pane, h("div", { class: "banner" }, h("span", {}, PS4() ? "Keys apply to shadPS4 (PS4) and AnyPS5. KytyPS5 and SharpEmu use their own fixed layout; the options below this list still apply to KytyPS5."
    : "Keys apply to AnyPS5. KytyPS5 and SharpEmu use their own fixed layout; the options below this list still apply to KytyPS5.")));
  const row = (label, help, ctl) => h("div", { class: "field" + (editable ? "" : " disabled") }, h("div", { class: "name" }, label), help ? h("div", { class: "help" }, help) : null, h("div", { class: "ctl" }, ctl));
  add(pane, 
    row("Let Strata set the keys", "Off = each engine keeps its own default keys. Your old files are kept as .bak.",
      h("button", { class: "switch" + (c.enabled ? " on" : ""), "data-nav": true, onclick: () => writeC((x) => (x.enabled = !x.enabled)) })),
    row("Swap Cross and Circle", "For controllers where you prefer the other confirm button.",
      h("button", { class: "switch" + (c.swap_cross_circle ? " on" : ""), "data-nav": true, onclick: () => writeC((x) => (x.swap_cross_circle = !x.swap_cross_circle)) })),
    row("Stick dead zone", "How far a stick moves before the game notices.",
      h("div", { class: "stepper" }, h("button", { "data-nav": true, onclick: () => writeC((x) => (x.deadzone = Math.max(0, x.deadzone - 2))) }, "−"), h("span", { class: "val" }, c.deadzone + " %"),
        h("button", { "data-nav": true, onclick: () => writeC((x) => (x.deadzone = Math.min(90, x.deadzone + 2))) }, "+"))));
  const grid = h("div", { class: "keymap" + (editable && c.enabled ? "" : " disabled") });
  for (const [action, label] of info.actions) {
    const sel = h("select", { "data-nav": true }, info.keys.map((k) => h("option", { value: k, selected: c.keys[action] === k }, keyLabel(k))));
    sel.addEventListener("change", () => writeC((x) => (x.keys[action] = sel.value)));
    grid.append(h("label", {}, h("span", {}, label), h("div", { class: "select" }, sel)));
  }
  add(pane, h("h4", { class: "sub" }, "Keyboard and mouse"), grid,
    h("div", { style: { marginTop: "14px" } }, h("button", { class: "btn small", "data-nav": true, onclick: () => writeC((x) => { x.keys = Object.fromEntries(info.actions.map(([a]) => [a, DEFAULT_KEYS[a]])); }) }, "Reset keys")));
}
const DEFAULT_KEYS = { cross: "space", circle: "c", square: "f", triangle: "r", l1: "q", r1: "e", l2: "mouse_right", r2: "mouse_left", l3: "lshift", r3: "v", options: "enter", touchpad: "tab", up: "up", down: "down", left: "left", right: "right", ls_up: "w", ls_down: "s", ls_left: "a", ls_right: "d", rs_up: "i", rs_down: "k", rs_left: "j", rs_right: "l" };
function keyLabel(k) {
  const names = { space: "Space", enter: "Enter", escape: "Esc", tab: "Tab", backspace: "Backspace", lshift: "Left Shift", rshift: "Right Shift", lctrl: "Left Ctrl", rctrl: "Right Ctrl", lalt: "Left Alt", ralt: "Right Alt", up: "↑", down: "↓", left: "←", right: "→", mouse_left: "Mouse left", mouse_right: "Mouse right", mouse_middle: "Mouse middle" };
  if (names[k]) return names[k];
  if (k.startsWith("kp")) return "Numpad " + k.slice(2);
  return k.toUpperCase();
}

function paneBackups(pane) {
  if (S.backups === null) { invoke("backup_list").then((b) => { S.backups = b; renderPane(); }).catch(() => (S.backups = [])); }
  const g = S.perGame ? S.games.find((x) => x.id === S.perGame) : null;
  let withShaders = false;
  const make = async (btn) => {
    btn.disabled = true;
    try {
      const p = await invoke("backup_create", { gameId: g ? g.id : null, shaders: withShaders });
      toast("Backup saved: " + p.split(/[\\/]/).pop(), "ok");
      S.backups = await invoke("backup_list");
    } catch (e) { toast(String(e), "err"); }
    btn.disabled = false;
    renderPane();
  };
  add(pane, 
    h("div", { class: "field" }, h("div", { class: "name" }, g ? `Back up ${g.title}` : "Back up all games"),
      h("div", { class: "help" }, g ? "Saves of this game from every engine." : "Saves from every engine, in one file."),
      h("div", { class: "ctl" }, h("button", { class: "btn small primary", "data-nav": true, onclick: (ev) => make(ev.currentTarget) }, "Back up now"))),
    h("div", { class: "field" }, h("div", { class: "name" }, "Include shader caches"), h("div", { class: "help" }, "Bigger file, but games skip the shader stutter after a restore."),
      h("div", { class: "ctl" }, h("button", { class: "switch", "data-nav": true, onclick: (ev) => { withShaders = !withShaders; ev.currentTarget.classList.toggle("on", withShaders); } }))),
    h("h4", { class: "sub" }, "Backups"));
  const list = (S.backups || []).filter((b) => !g || !g.title_id || b.title_id === g.title_id || !b.title_id);
  if (!list.length) add(pane, h("div", { class: "micro" }, S.backups === null ? "Loading…" : "No backups yet."));
  for (const b of list) {
    add(pane, h("div", { class: "field" }, h("div", { class: "name" }, b.name),
      h("div", { class: "help" }, `${b.title_id || "All games"} · ${b.folders} folder(s) · ${b.size_kb >= 1024 ? (b.size_kb / 1024).toFixed(1) + " MB" : b.size_kb + " KB"}`),
      h("div", { class: "ctl" }, h("button", { class: "btn small", "data-nav": true, onclick: async () => {
        const ok = await ask("Restore this backup?", "The saves in it replace the current ones. The current files are kept next to them as “.before-restore”.", "Restore");
        if (!ok) return;
        try { const n = await invoke("backup_restore", { file: b.file }); toast(`Restored ${n} file(s).`, "ok"); } catch (e) { toast(String(e), "err"); }
      } }, "Restore"))));
  }
  add(pane, h("div", { style: { marginTop: "14px" } }, h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.backups }).catch(() => toast("No backups yet.", "warn")); } }, "Open backups folder")));
}

function paneExtras(pane) {
  const g = S.games.find((x) => x.id === S.perGame);
  if (g.platform !== "ps4") {
    add(pane, h("div", { class: "banner" }, h("span", {}, "Updates, DLC, mods and patches are handled for PS4 games (shadPS4). PS5 engines read updates from the game folder itself.")));
    return;
  }
  const x = S.extras[g.id];
  if (!x) { invoke("game_extras", { id: g.id }).then((r) => { S.extras[g.id] = r; renderPane(); }).catch((e) => toast(String(e), "err")); add(pane, h("div", { class: "micro" }, "Checking…")); return; }
  const e = entry(g.id);
  const row = (name, help, ...ctl) => h("div", { class: "field" }, h("div", { class: "name" }, name), help ? h("div", { class: "help" }, help) : null, h("div", { class: "ctl" }, ...ctl));
  const open = (p) => invoke("open_path", { path: p }).catch((err) => toast(String(err), "err"));
  add(pane,
    row("Update", x.update ? `Installed${x.update_version ? " · version " + x.update_version : ""}. shadPS4 loads it automatically.` : "No update installed. Install the update .pkg and Strata puts it in the right place.",
      x.update ? h("button", { class: "btn small", "data-nav": true, onclick: () => open(x.update) }, "Open folder") : null,
      h("button", { class: "btn small", "data-nav": true, onclick: () => installPkg() }, "Install .pkg")),
    row("DLC", x.dlc.length ? `${x.dlc.length} installed: ${x.dlc.join(", ")}` : "No DLC installed.",
      h("button", { class: "btn small", "data-nav": true, onclick: () => installPkg() }, "Install DLC .pkg")),
    row("Mods", x.mods ? `${x.mods_files} file(s) in the mods folder. Files there replace the game's own files with the same path.` : "Put changed game files in the mods folder, keeping the game's folder structure. Turning mods off keeps the files.",
      h("button", { class: "switch" + (x.mods_enabled ? " on" : ""), "data-nav": true, title: "Use mods", onclick: async () => {
        try { await invoke("set_mods", { id: g.id, enabled: !x.mods_enabled }); delete S.extras[g.id]; renderPane(); } catch (err) { toast(String(err), "err"); }
      } }),
      x.mods ? h("button", { class: "btn small", "data-nav": true, onclick: () => open(x.mods) }, "Open folder") : null),
    row("Patch file", e.patch_file ? e.patch_file : "Game patches (XML, e.g. 60 FPS or resolution fixes) loaded by shadPS4 for this game.",
      h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("pick_patch"); if (p) { e.patch_file = p; save(); renderPane(); } } }, e.patch_file ? "Change" : "Choose file"),
      e.patch_file ? h("button", { class: "btn small ghost", "data-nav": true, onclick: () => { e.patch_file = ""; save(); renderPane(); } }, "Remove") : null));
}

function paneGameDev(pane) {
  const g = S.games.find((x) => x.id === S.perGame);
  const e = entry(S.perGame);
  const mine = logBuf.filter((l) => l.source === "engine" || l.source === "anyps5").slice(-60);
  const r = S.running && S.running.game_id === g.id ? S.running : null;
  add(pane, 
    h("div", { class: "facts" },
      h("div", {}, h("div", { class: "micro" }, "Folder"), h("div", { class: "v mono", style: { fontSize: "12px", wordBreak: "break-all" } }, g.path)),
      h("div", {}, h("div", { class: "micro" }, "Title ID · platform"), h("div", { class: "v" }, `${g.title_id || "—"} · ${g.platform.toUpperCase()}`)),
      h("div", {}, h("div", { class: "micro" }, "Engine that worked"), h("div", { class: "v" }, ENGINE_NAME[e.good_engine] || "—"))),
    r ? h("div", { class: "cmd" }, r.command) : null,
    h("div", { style: { display: "flex", gap: "8px", margin: "14px 0" } },
      h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.logs }); } }, "Open session logs"),
      h("button", { class: "btn small", "data-nav": true, onclick: async () => { try { const p = await invoke("export_diagnostics"); toast("Saved " + p, "ok"); } catch (x) { toast(String(x), "err"); } } }, "Export diagnostics"),
      h("button", { class: "btn small", "data-nav": true, onclick: () => go("dev") }, "Open full console")),
    h("h4", { class: "sub" }, "Latest engine log"),
    h("div", { class: "cmd", style: { maxHeight: "300px", overflow: "auto" } }, mine.length ? mine.map((l) => `${l.level.padEnd(5)} ${l.msg}`).join("\n") : "Nothing yet. Play the game to see its log here."));
}

function ask(title, text, okLabel = "OK", cancelLabel = "Cancel", extraLabel = null) {
  return new Promise((resolve) => {
    const m = h("div", { class: "modal show dialog" });
    const done = (v) => { m.remove(); nav.setScope(prevScope); resolve(v); };
    const prevScope = $("#wizard").classList.contains("show") ? $("#wizard .sheet") : null;
    m.append(h("div", { class: "sheet small" }, h("h2", {}, title), h("p", {}, text),
      h("div", { style: { display: "flex", gap: "10px", justifyContent: "flex-end" } },
        extraLabel ? h("button", { class: "btn ghost", "data-nav": true, style: { marginRight: "auto" }, onclick: () => done("extra") }, extraLabel) : null,
        h("button", { class: "btn", "data-nav": true, onclick: () => done(false) }, cancelLabel),
        h("button", { class: "btn primary", "data-nav": true, onclick: () => done(true) }, okLabel))));
    document.body.append(m);
    nav.setScope(m.querySelector(".sheet"));
    setTimeout(() => nav.focus(m.querySelector(".btn.primary"), { silent: true }), 40);
  });
}

function paneAppearance(pane) {
  const row = (key, label) => h("div", { class: "field" },
    h("div", { class: "name" }, label),
    h("div", { class: "help" }, key === "accent1" ? "Main buttons, progress and highlights." : "Focus rings and secondary highlights."),
    h("div", { class: "ctl" }, h("div", { class: "swatches" }, ACCENTS.map((a) =>
      h("button", { class: "sw" + (S.settings.ui[key] === a ? " on" : ""), "data-nav": true, title: a, style: { background: PALETTE[a] }, onclick: () => { S.settings.ui[key] = a; applyTheme(); save(); renderPane(); renderTop(); } })))));
  add(pane, row("accent1", "Accent 1"), row("accent2", "Accent 2"),
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
  add(pane, 
    h("div", { class: "pcard" },
      h("div", { style: { display: "flex", gap: "18px", alignItems: "center" } }, h("span", { html: MARK(), style: { width: "54px", height: "54px", display: "block" } }),
        h("div", {}, h("div", { class: "tier", style: { margin: 0 } }, "Strata"), h("div", { class: "micro" }, `Version ${S.boot.version}${S.boot.channel ? " " + S.boot.channel : ""} · ${S.boot.platform}`))),
      h("ul", { style: { marginTop: "20px" } },
        h("li", {}, `Engines: KytyPS5, SharpEmu, AnyPS5${PS4() ? ", shadPS4" : ""} — each GPL-2.0, by their own contributors, downloaded from their official GitHub releases.`),
        h("li", {}, "Strata launcher — GPL-2.0. Fonts: Archivo and IBM Plex Mono (SIL Open Font License)."),
        h("li", {}, "Strata is not affiliated with, endorsed by or connected to any console maker. It ships no games, firmware or keys."),
        h("li", {}, "Only play games you dumped yourself from hardware you own."),
        S.boot.channel ? h("li", {}, "This is a beta: expect rough edges. Developer → Export diagnostics creates a report you can share.") : null),
      h("div", { style: { display: "flex", gap: "10px" } },
        h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.data }); } }, "Open data folder"),
        h("button", { class: "btn small", "data-nav": true, onclick: () => { S.settings.first_run = true; save(); renderWizard(true); } }, "Run setup again"))));
  const links = Object.entries(SUPPORT).filter(([, url]) => url);
  if (links.length) {
    add(pane, h("div", { class: "pcard", style: { marginTop: "18px" } },
      h("div", { class: "tier", style: { margin: 0, fontSize: "20px" } }, "Support Strata"),
      h("p", { class: "lead" }, "Strata is free and has no ads. If it helps you, a small donation or a follow keeps it going."),
      h("div", { style: { display: "flex", gap: "10px", flexWrap: "wrap" } },
        links.map(([k, url]) => h("button", { class: "btn small" + (k === "kofi" ? " primary" : ""), "data-nav": true, onclick: () => invoke("open_url", { url }).catch((e) => toast(String(e), "err")) }, SUPPORT_LABEL[k])))));
  }
}

// ───────────────────────── community, Discord, TV mode ─────────────────────────
function toggleTv() {
  S.settings.ui.tv_mode = !S.settings.ui.tv_mode;
  applyTheme(); save();
  if (S.settings.ui.tv_mode) win.fullscreen();
  toast(S.settings.ui.tv_mode ? "TV mode on (F9 to leave)." : "TV mode off.", "ok");
  if (S.view === "settings") renderPane();
}

async function askShare() {
  if (S.settings.community.asked || document.querySelector(".dialog")) return;
  const yes = await ask("Help everyone find working games?",
    "Strata can send an anonymous result after you play: the game, the engine, whether it worked and what the engine was missing. No name, no files, no logs. Games that work appear on strata.gbxstudios.com/compatibility. You can change this any time in Settings → Community.",
    "Share results", "No thanks");
  S.settings.community.asked = true;
  S.settings.community.share = !!yes;
  save();
  toast(yes ? "Thanks! Results will be shared." : "Okay, nothing will be sent.", "ok");
}

function paneCommunity(pane) {
  const c = S.settings.community;
  const sw = (key, label, help, disabled = false) => h("div", { class: "field" + (disabled ? " disabled" : "") }, h("div", { class: "name" }, label), h("div", { class: "help" }, help),
    h("div", { class: "ctl" }, h("button", { class: "switch" + (c[key] ? " on" : ""), "data-nav": true, onclick: () => { if (disabled) return; c[key] = !c[key]; if (key === "share") c.asked = true; save(); renderPane(); } })));
  add(pane,
    sw("share", "Share test results", "After a game closes, send the game, engine, result and what the engine was missing. No name, no file paths, no logs. Working games appear on the website's Compatibility page; errors help fix the engines."),
    sw("use_results", "Use everyone's results", "Download the combined results (twice a day at most) so Strata starts games with the engine that worked for other players."),
    sw("discord", "Show on Discord", S.discordOk ? "Shows “Playing <game>” with the engine on your Discord profile while you play. Needs the Discord app open." : "Not available in this build yet.", !S.discordOk),
    h("div", { style: { display: "flex", gap: "10px", marginTop: "18px", flexWrap: "wrap" } },
      h("button", { class: "btn small", "data-nav": true, onclick: () => invoke("open_url", { url: "https://strata.gbxstudios.com/compatibility" }).catch((e) => toast(String(e), "err")) }, "Open the compatibility list"),
      h("button", { class: "btn small ghost", "data-nav": true, onclick: () => invoke("open_url", { url: "https://strata.gbxstudios.com/legal" }).catch((e) => toast(String(e), "err")) }, "What is sent, exactly")));
}

// ───────────────────────── per-game compatibility ─────────────────────────
const CHANCE = { worked: ["Worked", "ok"], failed: ["Failed", "bad"], high: ["High chance", "ok"], medium: ["Maybe", "warn"], low: ["Low chance", "bad"], unknown: ["Unknown", ""] };
function paneCompat(pane) {
  const g = S.games.find((x) => x.id === S.perGame);
  const c = S.compat?.[g.id];
  if (!c) {
    add(pane, h("div", { class: "micro" }, "Reading the game's program…"));
    invoke("game_compat", { id: g.id }).then((v) => { S.compat = { ...(S.compat || {}), [g.id]: v }; renderPane(); }).catch((e) => toast(String(e), "err"));
    return;
  }
  const a = c.record.analysis;
  add(pane,
    h("div", { class: "facts" },
      h("div", {}, h("div", { class: "micro" }, "Program"), h("div", { class: "v" }, a.readable ? `Readable (${a.format.toUpperCase()})` : a.format === "unknown" ? "Not found or unknown" : "Encrypted — not a decrypted dump")),
      h("div", {}, h("div", { class: "micro" }, "Made with"), h("div", { class: "v" }, a.game_engine || "Unknown")),
      h("div", {}, h("div", { class: "micro" }, "System functions used"), h("div", { class: "v" }, String(a.imports.length))),
      h("div", {}, h("div", { class: "micro" }, "System libraries"), h("div", { class: "v" }, String(a.libraries.length))),
      h("div", {}, h("div", { class: "micro" }, "Modules shipped with the game"), h("div", { class: "v" }, String(a.bundled.length))),
      h("div", {}, h("div", { class: "micro" }, "Tests recorded"), h("div", { class: "v" }, String(c.record.attempts.length)))),
    h("h4", { class: "sub" }, "Engines, best first"),
    c.scores.map((sc) => {
      const [label, kind] = CHANCE[sc.label] || [sc.label, ""];
      return h("div", { class: "field" },
        h("div", { class: "name" }, ENGINE_NAME[sc.engine] || sc.engine, h("span", { class: "chip " + kind, style: { marginLeft: "10px" } }, h("span", { class: "dot" }), label)),
        h("div", { class: "help" }, sc.reason, sc.known_missing.length ? h("div", { class: "mono", style: { fontSize: "11px", marginTop: "4px" } }, "Missing: " + sc.known_missing.slice(0, 8).join(", ") + (sc.known_missing.length > 8 ? " …" : "")) : null),
        h("div", { class: "ctl" }, h("span", { class: "micro" }, sc.score + " / 100")));
    }),
    c.record.attempts.length ? h("h4", { class: "sub" }, "Last tests") : null,
    c.record.attempts.slice(-8).reverse().map((t) => h("div", { class: "check " + (t.worked ? "pass" : "fail") }, h("span", { class: "st" }),
      h("div", {}, h("div", {}, `${ENGINE_NAME[t.engine] || t.engine} · ${t.worked ? "worked" : "failed"} after ${t.seconds} s`),
        h("div", { class: "d" }, [t.release, new Date(t.when * 1000).toLocaleString(), t.findings.missing_modules.length ? "missing " + t.findings.missing_modules.join(", ") : "", t.findings.missing_functions.length ? `${t.findings.missing_functions.length} missing function(s)` : ""].filter(Boolean).join(" · "))))),
    h("div", { style: { display: "flex", gap: "10px", marginTop: "18px", flexWrap: "wrap" } },
      h("button", { class: "btn small", "data-nav": true, onclick: async () => {
        try { await invoke("compat_reset", { id: g.id }); entry(g.id).good_engine = ""; delete S.compat[g.id]; renderPane(); toast("Next launch tests the engines again.", "ok"); } catch (e) { toast(String(e), "err"); }
      } }, "Test engines again"),
      h("button", { class: "btn small", "data-nav": true, onclick: async () => { const p = await invoke("paths"); invoke("open_path", { path: p.compat }).catch(() => toast("Nothing saved yet.", "warn")); } }, "Open compatibility files")),
    h("p", { class: "micro", style: { textTransform: "none", letterSpacing: 0, marginTop: "14px" } }, "The first launch reads the game and tests engines; after that Strata opens it straight in the engine that worked. The file for this game can be shared with friends: " + c.file.split(/[\\/]/).pop()));
}

// ───────────────────────── developer ─────────────────────────
const logBuf = [];
function renderDev() {
  invoke("skipped_folders").then((k) => { if (JSON.stringify(k) !== JSON.stringify(S.skipped)) { S.skipped = k; if (S.view === "dev") renderDev(); } }).catch(() => null);
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
        S.skipped.length ? h("div", { class: "box" }, h("h4", {}, "Skipped folders", h("span", { class: "micro" }, String(S.skipped.length))),
          S.skipped.map((k) => h("div", { class: "check warn" }, h("span", { class: "st" }), h("div", {}, h("div", { class: "mono", style: { fontSize: "12px", wordBreak: "break-all" } }, k.path), h("div", { class: "d" }, k.reason))))) : null,
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
  try {
    const info = await invoke("launch_game", { id });
    // the game may already have exited (instant crash) before this resolved
    S.running = (await invoke("running_game")) ? info : null;
    const e = entry(id);
    e.launches = (e.launches || 0) + 1;
    e.last_played = Math.floor(Date.now() / 1000);
    renderRunning();
    if (S.view === "home") { S.sel = 0; renderHome(); }
  } catch (err) {
    const m = String(err).match(/need-engine:(ps4|ps5)/);
    if (m) return needEngine(m[1], id);
    toast(String(err), "err");
  }
}
async function needEngine(platform, id) {
  const g = S.games.find((x) => x.id === id);
  const ps4 = platform === "ps4";
  const ok = await ask(ps4 ? "This is a PS4 game" : "No engine for this game yet",
    ps4 ? `${g?.title || "This game"} needs the PS4 engine (shadPS4, free and open source). Download it now? It takes a minute.`
      : "Strata needs a PS5 engine to start games. Download KytyPS5, SharpEmu and AnyPS5 now?",
    "Download");
  if (!ok) return;
  toast("Downloading… you can keep browsing.", "");
  if (ps4) await downloadEngine("shadps4"); else await downloadMany(PS5_SET);
  if (haveEngine(platform)) launch(id);
}
async function nextEngine() {
  try { await invoke("next_engine"); } catch (e) { toast(String(e), "warn"); }
}
async function stopGame() {
  try { await invoke("stop_game"); } catch (e) { toast(String(e), "err"); }
}
function renderRunning() {
  const el = $("#running");
  if (!S.running) { el.classList.add("hidden"); return; }
  el.classList.remove("hidden");
  const r = S.running;
  el.replaceChildren(...[h("span", { class: "pulse" }), h("span", { class: "t" }, r.title),
    r.engine_name ? h("span", { class: "micro eng" }, `${r.engine_name}${r.attempts > 1 ? ` · ${r.attempt} of ${r.attempts}` : ""}`) : null,
    h("span", { class: "time", id: "runtime" }, "0:00"),
    r.attempt < r.attempts ? h("button", { class: "btn small", "data-nav": true, title: "Close this engine and try the next one", onclick: nextEngine }, "Try next engine") : null,
    h("button", { class: "btn small danger", "data-nav": true, onclick: stopGame }, "Stop")].filter(Boolean));
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
    h("p", {}, `${PS4() ? "Four" : "Three"} quick steps. Everything can be changed later in Settings.`),
    h("div", { class: "steps" },
      step(1, !!S.settings.auto_profile, p ? `Your PC: ${p.label}` : "Checking your PC…",
        !p ? "" : p.tier === "unsupported" ? (p.reasons[0] || "Update your graphics driver.") : `${p.gpu_name ? shortGpu(p.gpu_name) + " · " : ""}${Math.round((S.sys?.ram_mb || 0) / 1024)} GB RAM — tuned settings ready.`,
        h("button", { class: "btn small", "data-nav": true, disabled: !p || p.tier === "unsupported", onclick: async () => { await applyProfile(); renderWizard(true); } }, S.settings.auto_profile ? "Applied" : "Apply")),
      (() => {
        const have = PS5_SET.filter((id) => S.engines.find((e) => e.id === id)?.path);
        const cur = PS5_SET.map((id) => S.downloads[id]).find((d) => d && d.stage !== "done" && d.stage !== "error");
        return step(2, have.length > 0, "PS5 engines",
          cur ? `${ENGINE_NAME[cur.engine]}: ${cur.stage}${cur.total ? ` · ${Math.round((cur.received / cur.total) * 100)}%` : ""}`
            : have.length ? `${have.map((id) => ENGINE_NAME[id]).join(", ")} ready.${have.length < 3 ? " The others can be added later." : ""}`
            : "Downloads KytyPS5, SharpEmu and AnyPS5 from their official GitHub releases. If one cannot run a game, Strata tries the next.",
          have.length === 3 ? h("span", { class: "micro" }, "Ready") : h("div", { style: { display: "flex", gap: "8px" } },
            h("button", { class: "btn small", "data-nav": true, disabled: !!cur, onclick: () => downloadMany(PS5_SET) }, have.length ? "Get the rest" : "Download all"),
            h("button", { class: "btn small", "data-nav": true, onclick: async () => {
              const p = await invoke("pick_folder");
              if (p) { S.settings.engines.paths.kyty = p; S.settings.engine_path = p; await save(true); await loadEngines(); renderWizard(true); renderTop(); if (!haveEngine("ps5")) toast("No KytyPS5 found in that folder.", "warn"); }
            } }, "Choose folder")));
      })(),
      !PS4() ? null : (() => {
        const ps4 = S.engines.find((e) => e.id === "shadps4");
        const d = S.downloads.shadps4;
        const skipped = S.settings.engines.skip_ps4;
        return step(3, !!ps4?.path || skipped, "PS4 engine (optional)",
          ps4?.path ? "shadPS4 ready. PS4 games start with it automatically."
            : d && d.stage !== "done" && d.stage !== "error" ? `shadPS4: ${d.stage}${d.total ? ` · ${Math.round((d.received / d.total) * 100)}%` : ""}`
            : skipped ? "Skipped. If you add a PS4 game later, Strata offers the download."
            : "Only if you play PS4 games too. Adds shadPS4, the most mature PS4 emulator.",
          ps4?.path ? h("span", { class: "micro" }, "Ready") : h("div", { style: { display: "flex", gap: "8px" } },
            h("button", { class: "btn small", "data-nav": true, disabled: busy("shadps4"), onclick: () => downloadEngine("shadps4") }, "Download"),
            skipped ? null : h("button", { class: "btn small ghost", "data-nav": true, onclick: () => { S.settings.engines.skip_ps4 = true; save(); renderWizard(true); } }, "I don't play PS4")));
      })(),
      step(PS4() ? 4 : 3, S.games.length > 0, "Your games", S.games.length ? `${S.games.length} found.` : "Put game folders in the Games folder, or add your own folder.",
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
    S.pkgs = await invoke("found_pkgs").catch(() => []);
    await loadStates();
    S.stats = null;
  } catch (e) { toast(String(e), "err"); }
  if (S.view === "home") renderHome();
  if (S.view === "library") renderLibrary();
}
async function loadEngine() {
  try { S.engine = await invoke("engine_info"); } catch (e) { S.engine = { path: null, version: null, error: String(e) }; }
}
async function loadEngines() {
  try { S.engines = await invoke("engines_status"); } catch (e) { S.engines = []; }
  await loadEngine();
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
  if (k === "f9") { toggleTv(); return true; }
  if (k === "f5") { refresh(); return true; }
  const games = sortedGames();
  if (S.view === "home" && games.length && !drawerFor) {
    const g = games[S.sel];
    if (k === "i" || k === "pad-y") openDrawer(g.id);
    else if (k === "f" || k === "pad-x") toggleFav(g.id);
    else if (k === "s") openGameSettings(g.id);
  } else if (S.view === "library" && k === "/") {
    e.preventDefault?.(); $("#search")?.focus();
  } else if (S.view === "library" && (k === "f" || k === "pad-x") && !drawerFor) {
    const id = document.activeElement?.dataset?.id;
    if (id) { toggleFav(id); toast(entry(id).favorite ? "Added to favorites." : "Removed from favorites.", "ok"); }
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
  const bootDone = playBoot({ style: u.boot_style, accent1: u.accent1, accent2: u.accent2, version: S.boot.version + (S.boot.channel ? " " + S.boot.channel : ""), reduceMotion: u.reduce_motion });

  // load everything while the boot plays
  listen("log", appendLog);
  listen("game-start", (r) => { S.running = r; renderRunning(); });
  listen("game-exit", (x) => {
    S.running = null;
    if (S.compat) delete S.compat[x.game_id];
    S.stats = null;
    loadStates().then(() => { if (S.view === "library") renderLibraryBody(); });
    if (!S.settings.community.asked) setTimeout(askShare, 1200);
    renderRunning();
    toast(`${S.games.find((g) => g.id === x.game_id)?.title || "Game"} — ${x.message}`, x.code && x.code !== 0 ? "err" : "ok");
    if (S.settings.games[x.game_id]) S.settings.games[x.game_id].play_seconds += x.seconds;
    if (S.view === "home") renderHome();
    if (S.view === "dev") renderDev();
  });
  listen("engine-download", (p) => {
    S.downloads[p.engine || "kyty"] = p;
    if (S.view === "settings" && S.section === "engine") renderPane();
    if ($("#wizard").classList.contains("show")) renderWizard(true);
  });
  listen("engine-fallback", (x) => toast(x.message, "warn"));
  listen("compat-updated", () => { S.compat = {}; if (S.view === "settings" && S.section === "compat") renderPane(); });
  listen("pkg-progress", (p) => {
    if (p.stage !== "extracting" || !S.pkgJob) return;
    S.pkgJob = p;
    const bar = $("#pkgbar");
    if (bar && S.view === "library") bar.replaceWith(pkgBanner());
  });
  listen("engine-status", (x) => toast(x.message, ""));
  const [games] = await Promise.all([
    invoke("scan_games").catch(() => []),
    invoke("system_info", { refresh: false }).then((s) => (S.sys = s)).catch(() => null),
    loadEngines(),
    invoke("monitor_refresh").then((hz) => (S.monitorHz = hz)).catch(() => null),
    invoke("logs_since", { seq: 0 }).then((ls) => ls.forEach(appendLog)).catch(() => null),
  ]);
  S.games = games;
  S.pkgs = await invoke("found_pkgs").catch(() => []);
  await loadStates();
  S.discordOk = await invoke("discord_available").catch(() => false);
  S.profile = await invoke("auto_profile").catch(() => null);
  S.loaded = true;
  log("info", `UI ready · ${S.games.length} games · engines: ${S.engines.filter((e) => e.path).map((e) => e.name).join(", ") || "none"}`);
  await bootDone;
  renderRunning();
  go("home");
  if (S.settings.first_run) renderWizard();
})();
