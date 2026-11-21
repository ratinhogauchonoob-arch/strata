// Bridge to the Rust side. In a plain browser (design preview / website demo) it falls
// back to a mock so the interface can be explored without the app.

const T = window.__TAURI__;
export const isApp = !!(T && T.core);

export function invoke(cmd, args = {}) {
  if (isApp) return T.core.invoke(cmd, args);
  return mock(cmd, args);
}

export function listen(event, cb) {
  if (isApp) return T.event.listen(event, (e) => cb(e.payload));
  mockListeners[event] = (mockListeners[event] || []).concat(cb);
  return Promise.resolve(() => {});
}

export function fileSrc(path) {
  if (!path) return "";
  if (isApp) return T.core.convertFileSrc(path);
  return path;
}

export const win = {
  minimize: () => isApp && T.window.getCurrentWindow().minimize(),
  toggleMax: () => isApp && T.window.getCurrentWindow().toggleMaximize(),
  close: () => isApp && T.window.getCurrentWindow().close(),
  drag: () => isApp && T.window.getCurrentWindow().startDragging(),
  fullscreen: async () => {
    if (!isApp) return;
    const w = T.window.getCurrentWindow();
    await w.setFullscreen(!(await w.isFullscreen()));
  },
};

// ───────────────────────── mock ─────────────────────────
const mockListeners = {};
const emit = (ev, p) => (mockListeners[ev] || []).forEach((f) => f(p));
let seq = 0;
const mlogs = [];
function mlog(level, source, msg) {
  const d = new Date();
  const t = d.toTimeString().slice(0, 8) + "." + String(d.getMilliseconds()).padStart(3, "0");
  const line = { seq: ++seq, t, level, source, msg };
  mlogs.push(line);
  emit("log", line);
}

function svgArt(text, a, b) {
  const s = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 512 512'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop stop-color='${a}'/><stop offset='1' stop-color='${b}'/></linearGradient></defs><rect width='512' height='512' fill='url(#g)'/><g fill='rgba(255,255,255,.14)'><rect x='-40' y='300' width='620' height='26' transform='rotate(-14 256 256)'/><rect x='-40' y='350' width='620' height='12' transform='rotate(-14 256 256)'/></g><text x='40' y='470' font-family='Arial Black,Arial' font-size='64' font-weight='900' fill='rgba(255,255,255,.9)'>${text}</text></svg>`;
  return "data:image/svg+xml;utf8," + encodeURIComponent(s);
}
function wideArt(a, b) {
  const s = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1920 1080'><defs><radialGradient id='r' cx='.72' cy='.38' r='.7'><stop stop-color='${a}'/><stop offset='1' stop-color='${b}'/></radialGradient></defs><rect width='1920' height='1080' fill='url(#r)'/><g fill='rgba(0,0,0,.25)'><circle cx='1500' cy='420' r='260'/><rect x='0' y='760' width='1920' height='320'/></g></svg>`;
  return "data:image/svg+xml;utf8," + encodeURIComponent(s);
}

const demoGames = [
  ["Night Courier", "DEMO00101", "#3a2a6b", "#0e0b1f", "playable"],
  ["Salt & Iron", "DEMO00214", "#7a3b1d", "#1a0d06", "menus"],
  ["Lumen Drift", "DEMO00377", "#155b63", "#04161a", "playable"],
  ["Paper Kingdoms", "DEMO00402", "#6b6418", "#141203", "boots"],
  ["Hollow Signal", "DEMO00518", "#3b4252", "#0b0d12", "unknown"],
  ["Tidebreaker", "DEMO00620", "#1d4a7a", "#050f1c", "broken"],
  ["Ashfall Run", "DEMO00733", "#7a1d2c", "#1a0509", "unknown"],
].map(([title, id, a, b, status], i) => ({
  id: "demo/" + id,
  path: "D:/Games/" + title,
  title,
  title_id: id,
  version: "01.00" + i,
  firmware: "4.50",
  archive: i === 6,
  icon: svgArt(title.split(" ")[0].toUpperCase().slice(0, 7), a, b),
  background: wideArt(a, b),
  size_mb: 12000 + i * 3100,
  folder: "D:/Games",
  platform: i === 4 || i === 5 ? "ps4" : "ps5",
  _status: status,
}));
const mEngines = {
  kyty: { name: "KytyPS5", repo: "KytyPS5/KytyPS5", platforms: ["ps5"], archives: true, summary: "PS5 translation layer. Goes in-game in many titles. Uses every setting in Strata.", path: "C:/Strata/data/engine/KytyPS5-Windows-x64/kyty_emulator.exe" },
  sharpemu: { name: "SharpEmu", repo: "sharpemu/sharpemu", platforms: ["ps5", "ps4"], archives: true, summary: "PS5 emulator written in C#, with an experimental PS4 mode.", path: null },
  anyps5: { name: "AnyPS5", repo: "boykopovar/AnyPS5", platforms: ["ps5"], archives: false, summary: "Converts the game into a native program once, then runs it. Very early.", path: null },
  shadps4: { name: "shadPS4", repo: "shadps4-emu/shadPS4", platforms: ["ps4"], archives: false, summary: "The most mature PS4 emulator. Hundreds of games are playable.", path: null },
};
const MACTIONS = [["cross","Cross"],["circle","Circle"],["square","Square"],["triangle","Triangle"],["l1","L1"],["r1","R1"],["l2","L2"],["r2","R2"],["l3","L3"],["r3","R3"],["options","Options"],["touchpad","Touchpad"],["up","D-pad up"],["down","D-pad down"],["left","D-pad left"],["right","D-pad right"],["ls_up","Left stick up"],["ls_down","Left stick down"],["ls_left","Left stick left"],["ls_right","Left stick right"],["rs_up","Right stick up"],["rs_down","Right stick down"],["rs_left","Right stick left"],["rs_right","Right stick right"]];
const MKEYS = { cross: "space", circle: "c", square: "f", triangle: "r", l1: "q", r1: "e", l2: "mouse_right", r2: "mouse_left", l3: "lshift", r3: "v", options: "enter", touchpad: "tab", up: "up", down: "down", left: "left", right: "right", ls_up: "w", ls_down: "s", ls_left: "a", ls_right: "d", rs_up: "i", rs_down: "k", rs_left: "j", rs_right: "l" };
const mBackups = [];

let mset = null;
function defaults() {
  return {
    version: 2, first_run: true, engine_path: "",
    engines: { order_ps5: ["kyty", "sharpemu", "anyps5"], order_ps4: ["shadps4", "sharpemu"], auto_fallback: true, fail_seconds: 20, paths: {}, skip_ps4: false },
    controls: { enabled: false, keys: { ...MKEYS }, swap_cross_circle: false, deadzone: 8 }, library_folders: ["C:/Strata/data/Games", "D:/Games"],
    ui: { accent1: "ember", accent2: "cobalt", boot_style: "duo", reduce_motion: false, clock_24h: true, ui_sounds: true },
    emu: {
      resolution: "1280x720", present_mode: "Mailbox", gpu_index: -1, fullscreen: false, stretch: false, hdr: false, hide_cursor: true,
      vblank_frequency: 0, shader_cache: true, relaxed_shader_float: false, readback_linear: false, readback_tiled: false,
      readback_storage: false, sync_raw_image_buffers: false, flush_draws: 0, tessellation: false, occlusion_queries: false,
      native_rt: false, pro_mode: false, upscaler: "Linear", dlss_folder: "", fsr_folder: "", amd_cpu_compat: false,
      guest_tsc: false, red_zone: false, user_name: "Player", user_id: 1, console_language: 1, network: false,
      emulate_online: false, skip_notice_screen: true, trophies: true, trophy_duration: 6, perf_overlay: false,
      keyboard_player: 0, vibration: 100, speaker_volume: 50, controller_color: "", vulkan_validation: false,
      shader_validation: false, shader_log: "Silent", command_buffer_dump: false, profiler: false, renderdoc: false,
    },
    perf: { ram_limit_enabled: false, ram_limit_gb: 12, ram_mode: "soft", priority: "normal" },
    dev: { enabled: true, log_level: "info", capture_engine_log: true, keep_sessions: 20, show_command_line: false },
    auto_profile: "",
    games: Object.fromEntries(demoGames.map((g, i) => [g.id, { favorite: i === 2, last_played: null, play_seconds: i * 4000, launches: i, status: g._status, note: "", emu: null, perf: null, engine: "", good_engine: i === 1 ? "kyty" : "", controls: null }])),
  };
}
const demoSys = {
  os: "Windows 11 (26100)", cpu: "13th Gen Intel(R) Core(TM) i5-13420H", cpu_vendor: "GenuineIntel", cores: 8, threads: 12, ram_mb: 16 * 1024,
  gpus: [
    { index: 0, name: "Intel(R) UHD Graphics", kind: "integrated", vram_mb: 1024, vulkan: "1.3.289", vulkan_ok: true, vendor: "Intel", driver: "" },
    { index: 1, name: "NVIDIA GeForce RTX 4050 Laptop GPU", kind: "discrete", vram_mb: 6141, vulkan: "1.4.303", vulkan_ok: true, vendor: "NVIDIA", driver: "566.36" },
  ],
  vulkan_error: null,
};
const demoProfile = {
  tier: "high", label: "High · 1080p", resolution: "1920x1080", present_mode: "Mailbox", gpu_index: 1,
  gpu_name: "NVIDIA GeForce RTX 4050 Laptop GPU", relaxed_shader_float: false, ram_limit_gb: 12, amd_cpu_compat: true,
  reasons: ["NVIDIA GeForce RTX 4050 Laptop GPU is a known GPU (class 3).", "Several GPUs found; pinned to #1 (NVIDIA GeForce RTX 4050 Laptop GPU).", "Intel CPU detected: CPU compatibility patches enabled.", "RAM cap suggestion: 12 GB of 16 GB."],
};
let mrun = null;

async function mock(cmd, a) {
  await new Promise((r) => setTimeout(r, 60));
  if (!mset) {
    mset = defaults();
    mlog("info", "app", "Strata (browser preview) — no engine attached");
  }
  switch (cmd) {
    case "bootstrap":
      return { app: "Strata", version: "0.9.0", channel: "beta", platform: "web", data_dir: "C:/Strata/data", games_dir: "C:/Strata/data/Games", settings: structuredClone(mset), engine: "C:/Strata/data/engine/KytyPS5-Windows-x64/kyty_emulator.exe", running: mrun };
    case "save_settings": mset = structuredClone(a.settings); mlog("debug", "settings", "saved"); return structuredClone(mset);
    case "system_info": return demoSys;
    case "auto_profile": return demoProfile;
    case "apply_auto_profile":
      Object.assign(mset.emu, { resolution: "1920x1080", gpu_index: 1, amd_cpu_compat: true });
      mset.perf.ram_limit_gb = 12; mset.auto_profile = "high"; mset.first_run = false;
      mlog("info", "profile", "Applied High · 1080p"); return structuredClone(mset);
    case "scan_games": mlog("info", "library", `${demoGames.length} game(s) found in 41 ms`); return demoGames;
    case "game_size": return 23456;
    case "pick_folder": return "E:/PS5Games";
    case "pick_file": return "C:/Engine/kyty_emulator.exe";
    case "open_path": return null;
    case "launch_game": {
      const g = demoGames.find((x) => x.id === a.id);
      const order = g.platform === "ps4" ? mset.engines.order_ps4 : mset.engines.order_ps5;
      const chain = order.filter((id) => mEngines[id].path && mEngines[id].platforms.includes(g.platform));
      if (!chain.length) throw "need-engine:" + g.platform;
      mrun = { game_id: g.id, engine: chain[0], engine_name: mEngines[chain[0]].name, attempt: 1, attempts: chain.length, title: g.title, pid: 4242, started_ms: Date.now(), command: "kyty_emulator.exe --screen-width 1920 --screen-height 1080 … --game \"" + g.path + "/eboot.bin\"", notes: ["Soft RAM limit: 12 GB of physical memory (extra goes to the page file)."], session_log: "C:/Strata/data/logs/session.log" };
      mlog("info", "engine", "Starting " + g.title);
      const lines = ["Vulkan: NVIDIA GeForce RTX 4050 Laptop GPU", "Loading eboot.bin", "WARN unimplemented: sceNpCheckCallback", "shader cache: 412 pipelines loaded", "frame 60 · 16.6 ms"];
      lines.forEach((l, i) => setTimeout(() => mlog(l.startsWith("WARN") ? "warn" : "info", "engine", l), 400 * (i + 1)));
      setTimeout(() => emit("game-start", mrun), 10);
      return mrun;
    }
    case "stop_game": {
      const r = mrun; mrun = null;
      setTimeout(() => emit("game-exit", { game_id: r?.game_id, code: null, seconds: 12, message: "The game was stopped." }), 200);
      return null;
    }
    case "running_game": return mrun;
    case "logs_since": return mlogs.filter((l) => l.seq > a.seq);
    case "clear_logs": mlogs.length = 0; return null;
    case "ui_log": mlog(a.level, "ui", a.msg); return null;
    case "run_selftest":
      return [
        ["Data folder", "pass", "C:/Strata/data is writable"], ["Free disk space", "pass", "212 GB free on C:\\"],
        ["Engine found", "pass", "C:/Strata/data/engine/KytyPS5-Windows-x64/kyty_emulator.exe"],
        ["Engine responds", "pass", "Release, clang-lld_link, ver = 0.3.0"], ["Vulkan loader", "pass", "2 GPU(s) visible"],
        ["Vulkan 1.3 GPU", "pass", "#1 NVIDIA GeForce RTX 4050 Laptop GPU · Vulkan 1.4.303 · 6141 MB"], ["Selected GPU", "pass", "#1"],
        ["Auto profile", "pass", "High · 1080p"], ["Memory", "pass", "16 GB installed"], ["Library folders", "warn", "Not reachable: D:/Games"],
        ["Games", "pass", "7 found"], ["Command line", "pass", "52 arguments"],
      ].map(([name, status, detail]) => ({ name, status, detail, ms: Math.round(Math.random() * 30) }));
    case "engine_info": return { path: "C:/Strata/data/engine/KytyPS5-Windows-x64/kyty_emulator.exe", version: "Release, clang-lld_link, ver = 0.3.0, git = a1fc5ae, date = Nov 20 2026", error: null };
    case "download_engine": {
      const id = a.id || "kyty";
      for (let i = 1; i <= 10; i++) setTimeout(() => emit("engine-download", { stage: i < 10 ? "downloading" : "done", received: i * 9e6, total: 9e7, message: id + ".zip", engine: id }), i * 120);
      await new Promise((r) => setTimeout(r, 1300));
      mEngines[id].path = `C:/Strata/data/engines/${id}/bin/${id}-release/${id}.exe`;
      return mEngines[id].path;
    }
    case "engines_status": return Object.entries(mEngines).map(([id, e]) => ({ id, ...e, release: e.path ? e.path.split("/").slice(-2, -1)[0] : null, controls: id === "shadps4" || id === "anyps5" }));
    case "next_engine": {
      if (!mrun || mrun.attempt >= mrun.attempts) throw "This was the last engine for this game.";
      emit("engine-fallback", { game_id: mrun.game_id, message: `${mrun.engine_name} skipped — trying the next engine…` });
      mrun = { ...mrun, attempt: mrun.attempt + 1, engine_name: "SharpEmu", started_ms: Date.now() };
      setTimeout(() => emit("game-start", mrun), 300);
      return null;
    }
    case "monitor_refresh": return 280;
    case "skipped_folders": return [{ path: "E:/PS5Games/backport (4.xx to 10.xx)", reason: "has eboot.bin but no sce_sys/param.json or param.sfo (not a game dump)" }];
    case "control_info": return { actions: MACTIONS, keys: [..."abcdefghijklmnopqrstuvwxyz0123456789"].concat(Array.from({ length: 12 }, (_, i) => "f" + (i + 1)), Array.from({ length: 10 }, (_, i) => "kp" + i), ["space","enter","escape","tab","backspace","lshift","rshift","lctrl","rctrl","lalt","ralt","up","down","left","right","mouse_left","mouse_right","mouse_middle"]) };
    case "backup_list": return mBackups;
    case "backup_create": {
      const g = demoGames.find((x) => x.id === a.gameId);
      const b = { file: `C:/Strata/data/backups/20261121-${mBackups.length}.zip`, name: `20261121-131${mBackups.length}-${(g?.title || "All games").replace(/\W/g, "_")}`, size_kb: 2048, created: "20261121", title_id: g?.title_id || "", folders: 2 };
      mBackups.unshift(b);
      return b.file;
    }
    case "backup_restore": return 14;
    case "export_diagnostics": return "C:/Strata/data/logs/diagnostics-20261120.txt";
    case "paths": return { data: "C:/Strata/data", logs: "C:/Strata/data/logs", games: "C:/Strata/data/Games", engine: "C:/Strata/data/engine", engines: "C:/Strata/data/engines", backups: "C:/Strata/data/backups" };
  }
  throw new Error("mock: unknown command " + cmd);
}
