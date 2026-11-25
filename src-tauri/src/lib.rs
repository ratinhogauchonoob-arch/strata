//! Strata — a launcher frontend. The UI lives in ../ui; this side talks to the OS and the engine.

pub mod backups;
pub mod community;
pub mod compat;
pub mod controls;
pub mod discord;
mod update;
pub mod display;
pub mod download;
pub mod engine;
pub mod engines;
pub mod games;
pub mod gpu;
pub mod logs;
pub mod memcap;
pub mod pkginstall;
pub mod profile;
pub mod selftest;
pub mod settings;

use serde::Serialize;
use settings::Settings;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

pub const APP_NAME: &str = "Strata";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_CHANNEL: &str = "beta";
/// PS4 support (shadPS4, PS4 games, .pkg install) is finished in code but held back from
/// this release while it is tested. Flip to true to turn it on everywhere.
pub const PS4_ENABLED: bool = false;

/// When Strata started (for the Discord "elapsed" timer in the menus).
pub static STARTED_UNIX: std::sync::OnceLock<i64> = std::sync::OnceLock::new();

fn now_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Discord shows the menus when no game runs (only with "Show on Discord" on).
fn discord_idle(state: &AppState) {
    if state.settings.lock().unwrap().community.discord && state.running.lock().unwrap().is_none() {
        discord::idle(state.games.lock().unwrap().len(), *STARTED_UNIX.get_or_init(now_unix));
    }
}

pub struct AppState {
    pub data: PathBuf,
    pub settings: Arc<Mutex<Settings>>,
    pub running: Arc<Mutex<Option<engine::Running>>>,
    pub sys: Mutex<Option<gpu::SystemInfo>>,
    pub games: Mutex<Vec<games::Game>>,
    pub skipped: Mutex<Vec<games::Skipped>>,
    pub pkgs: Mutex<Vec<String>>,
}

impl AppState {
    fn settings_path(&self) -> PathBuf {
        self.data.join("settings.json")
    }
    fn save(&self, s: &Settings) -> Result<(), String> {
        s.save(&self.settings_path()).map_err(|e| format!("could not save settings: {e}"))
    }
    fn default_games_dir(&self) -> PathBuf {
        self.data.join("Games")
    }
}

#[derive(Serialize)]
struct Bootstrap {
    app: &'static str,
    version: &'static str,
    channel: &'static str,
    platform: &'static str,
    data_dir: String,
    games_dir: String,
    settings: Settings,
    engine: Option<String>,
    running: Option<engine::RunningInfo>,
    ps4: bool,
}

#[tauri::command]
fn bootstrap(state: State<AppState>) -> Bootstrap {
    let games_dir = state.default_games_dir();
    let _ = std::fs::create_dir_all(&games_dir);
    let mut s = state.settings.lock().unwrap();
    let gd = games_dir.to_string_lossy().into_owned();
    if s.first_run && !s.library_folders.contains(&gd) {
        s.library_folders.insert(0, gd.clone());
        let _ = state.save(&s);
    }
    let engine = engines::ENGINES
        .iter()
        .find_map(|d| engines::find(d, &s.engine_path(d.id), &state.data))
        .map(|p| p.to_string_lossy().into_owned());
    Bootstrap {
        app: APP_NAME,
        version: APP_VERSION,
        channel: APP_CHANNEL,
        platform: std::env::consts::OS,
        data_dir: state.data.to_string_lossy().into_owned(),
        games_dir: gd,
        settings: s.clone(),
        engine,
        running: state.running.lock().unwrap().as_ref().map(|r| r.info.clone()),
        ps4: PS4_ENABLED,
    }
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    let mut new = settings;
    new.sanitize();
    logs::set_level(&new.dev.log_level);
    let was = state.settings.lock().unwrap().community.discord;
    state.save(&new)?;
    *state.settings.lock().unwrap() = new.clone();
    if new.community.discord != was {
        if new.community.discord { discord_idle(&state); } else { discord::clear(); }
    }
    logs::debug("settings", "saved");
    Ok(new)
}

#[tauri::command]
async fn system_info(state: State<'_, AppState>, refresh: bool) -> Result<gpu::SystemInfo, String> {
    if !refresh {
        if let Some(s) = state.sys.lock().unwrap().clone() {
            return Ok(s);
        }
    }
    let info = tauri::async_runtime::spawn_blocking(gpu::system_info).await.map_err(|e| e.to_string())?;
    logs::info(
        "system",
        &format!(
            "{} · {} · {} GB RAM · GPUs: {}",
            info.os,
            info.cpu,
            info.ram_mb / 1024,
            info.gpus.iter().map(|g| format!("#{} {} ({} MB, Vulkan {})", g.index, g.name, g.vram_mb, g.vulkan)).collect::<Vec<_>>().join(", ")
        ),
    );
    if let Some(e) = &info.vulkan_error {
        logs::warn("system", e);
    }
    *state.sys.lock().unwrap() = Some(info.clone());
    Ok(info)
}

#[tauri::command]
async fn auto_profile(state: State<'_, AppState>) -> Result<profile::AutoProfile, String> {
    let sys = system_info(state, false).await?;
    Ok(profile::compute(&sys))
}

#[tauri::command]
async fn apply_auto_profile(state: State<'_, AppState>) -> Result<Settings, String> {
    let sys = system_info(state.clone(), false).await?;
    let p = profile::compute(&sys);
    let mut guard = state.settings.lock().unwrap();
    let s = &mut *guard;
    profile::apply(&p, &mut s.emu, &mut s.perf);
    s.auto_profile = p.tier.clone();
    s.first_run = false;
    state.save(&s)?;
    logs::info("profile", &format!("Applied {} ({})", p.label, p.reasons.join(" ")));
    Ok(s.clone())
}

#[tauri::command]
async fn scan_games(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<games::Game>, String> {
    let folders = state.settings.lock().unwrap().library_folders.clone();
    let t = std::time::Instant::now();
    let (mut list, mut skipped, mut pkgs) = tauri::async_runtime::spawn_blocking(move || games::scan_full(&folders, false)).await.map_err(|e| e.to_string())?;
    if !PS4_ENABLED {
        for g in list.iter().filter(|g| g.platform == "ps4") {
            skipped.push(games::Skipped { path: g.path.clone(), reason: "PS4 game: PS4 support comes in a later version".into() });
        }
        list.retain(|g| g.platform != "ps4");
        pkgs.clear();
    }
    if !pkgs.is_empty() {
        logs::info("library", &format!("{} .pkg file(s) waiting to be installed", pkgs.len()));
    }
    *state.pkgs.lock().unwrap() = pkgs;
    logs::info("library", &format!("{} game(s) found in {} ms", list.len(), t.elapsed().as_millis()));
    for sk in &skipped {
        logs::debug("library", &format!("Skipped {}: {}", sk.path, sk.reason));
    }
    *state.games.lock().unwrap() = list.clone();
    *state.skipped.lock().unwrap() = skipped;
    discord_idle(&state);
    // Read new or updated games' eboot.bin in the background (first launch is then faster).
    {
        let (data, games, app2) = (state.data.clone(), list.clone(), app.clone());
        std::thread::spawn(move || {
            let mut n = 0;
            for g in games.iter().filter(|g| !g.archive) {
                let rec = compat::load(&data, &compat_key(g));
                if !compat::is_fresh(&rec.analysis, std::path::Path::new(&g.path)) {
                    ensure_record(&data, g);
                    n += 1;
                }
            }
            if n > 0 {
                logs::info("compat", &format!("Analyzed {n} game(s) in the background"));
                let _ = app2.emit("compat-updated", n);
            }
        });
    }
    Ok(list)
}

#[derive(Serialize)]
struct CompatView {
    record: compat::GameRecord,
    scores: Vec<compat::Score>,
    file: String,
}

#[tauri::command]
async fn game_compat(state: State<'_, AppState>, id: String) -> Result<CompatView, String> {
    let g = state.games.lock().unwrap().iter().find(|g| g.id == id).cloned().ok_or("Game not found.")?;
    let (s, data) = (state.settings.lock().unwrap().clone(), state.data.clone());
    tauri::async_runtime::spawn_blocking(move || {
        let rec = ensure_record(&data, &g);
        let k = compat::load_knowledge(&data);
        let mut scores = engine_scores(&s, &data, &g, &rec, &k);
        scores.sort_by(|a, b| b.score.cmp(&a.score));
        CompatView { file: compat::record_path(&data, &compat_key(&g)).to_string_lossy().into_owned(), record: rec, scores }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Forgets the test results of a game (the analysis stays): next launch tests the engines again.
#[tauri::command]
fn compat_reset(state: State<AppState>, id: String) -> Result<(), String> {
    let g = state.games.lock().unwrap().iter().find(|g| g.id == id).cloned().ok_or("Game not found.")?;
    let key = compat_key(&g);
    let mut rec = compat::load(&state.data, &key);
    rec.attempts.clear();
    compat::save(&state.data, &key, &rec)?;
    let mut s = state.settings.lock().unwrap();
    if let Some(e) = s.games.get_mut(&id) {
        e.good_engine.clear();
    }
    state.save(&s)?;
    logs::info("compat", &format!("Test results for {} cleared", g.title));
    Ok(())
}

/// Per game: "worked" | "failed" | "untested" and the engine that worked (for Library filters).
#[derive(Serialize)]
struct GameState {
    state: &'static str,
    engine: String,
}

fn game_state(rec: &compat::GameRecord) -> GameState {
    if let Some(a) = rec.attempts.iter().rev().find(|a| a.worked) {
        GameState { state: "worked", engine: a.engine.clone() }
    } else if !rec.attempts.is_empty() {
        GameState { state: "failed", engine: String::new() }
    } else {
        GameState { state: "untested", engine: String::new() }
    }
}

#[tauri::command]
fn library_states(state: State<AppState>) -> std::collections::BTreeMap<String, GameState> {
    let games = state.games.lock().unwrap().clone();
    games.iter().map(|g| (g.id.clone(), game_state(&compat::load(&state.data, &compat_key(g))))).collect()
}

#[derive(Serialize)]
struct TopGame {
    id: String,
    title: String,
    seconds: u64,
    launches: u32,
}

#[derive(Serialize)]
struct Stats {
    games: usize,
    total_seconds: u64,
    launches: u32,
    worked: usize,
    failed: usize,
    untested: usize,
    top: Vec<TopGame>,
    /// engine id → (tests, times it worked)
    engines: std::collections::BTreeMap<String, (u32, u32)>,
    /// "Unreal Engine" → number of games
    made_with: std::collections::BTreeMap<String, u32>,
}

#[tauri::command]
fn stats(state: State<AppState>) -> Stats {
    let games = state.games.lock().unwrap().clone();
    let s = state.settings.lock().unwrap().clone();
    let mut st = Stats { games: games.len(), total_seconds: 0, launches: 0, worked: 0, failed: 0, untested: 0, top: vec![], engines: Default::default(), made_with: Default::default() };
    for g in &games {
        let e = s.games.get(&g.id).cloned().unwrap_or_default();
        st.total_seconds += e.play_seconds;
        st.launches += e.launches;
        if e.play_seconds > 0 || e.launches > 0 {
            st.top.push(TopGame { id: g.id.clone(), title: g.title.clone(), seconds: e.play_seconds, launches: e.launches });
        }
        let rec = compat::load(&state.data, &compat_key(g));
        match game_state(&rec).state {
            "worked" => st.worked += 1,
            "failed" => st.failed += 1,
            _ => st.untested += 1,
        }
        for a in &rec.attempts {
            let slot = st.engines.entry(a.engine.clone()).or_default();
            slot.0 += 1;
            if a.worked {
                slot.1 += 1;
            }
        }
        if !rec.analysis.game_engine.is_empty() {
            *st.made_with.entry(rec.analysis.game_engine.clone()).or_default() += 1;
        }
    }
    st.top.sort_by(|a, b| b.seconds.cmp(&a.seconds).then(b.launches.cmp(&a.launches)));
    st.top.truncate(10);
    st
}

#[tauri::command]
async fn check_update() -> Result<update::UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(update::check).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn discord_available() -> bool {
    discord::available()
}

#[tauri::command]
fn found_pkgs(state: State<AppState>) -> Vec<String> {
    state.pkgs.lock().unwrap().clone()
}

/// Where installed games go: the chosen folder, else a library folder outside Strata's own
/// data (usually the user's games drive), else Strata's Games folder.
pub fn pkg_target(s: &Settings, default_games: &std::path::Path) -> PathBuf {
    let chosen = PathBuf::from(s.pkg_install_folder.trim());
    if !s.pkg_install_folder.trim().is_empty() && chosen.is_dir() {
        return chosen;
    }
    let own = games::normalize(default_games);
    s.library_folders
        .iter()
        .find(|f| games::normalize(std::path::Path::new(f)) != own && std::path::Path::new(f).is_dir())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_games.to_path_buf())
}

#[tauri::command]
fn pkg_target_folder(state: State<AppState>) -> String {
    let s = state.settings.lock().unwrap();
    pkg_target(&s, &state.default_games_dir()).to_string_lossy().into_owned()
}

fn shadps4_user(state: &AppState) -> PathBuf {
    engines::root_dir(&state.data, "shadps4").join("user")
}

#[tauri::command]
async fn pick_pkg(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().add_filter("PS4 package", &["pkg"]).blocking_pick_file().map(|p| p.to_string())
}

#[tauri::command]
async fn pick_patch(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().add_filter("Patch file", &["xml"]).blocking_pick_file().map(|p| p.to_string())
}

#[derive(Serialize)]
struct PkgPlan {
    info: pkginstall::PkgInfo,
    destination: String,
    exists: bool,
    game_found: bool,
}

/// Reads a package and says where it would go (nothing is written).
#[tauri::command]
async fn pkg_inspect(state: State<'_, AppState>, path: String, folder: Option<String>) -> Result<PkgPlan, String> {
    let p = path.clone();
    let info = tauri::async_runtime::spawn_blocking(move || pkginstall::inspect(std::path::Path::new(&p))).await.map_err(|e| e.to_string())??;
    let library = folder.map(PathBuf::from).unwrap_or_else(|| {
        let s = state.settings.lock().unwrap();
        pkg_target(&s, &state.default_games_dir())
    });
    let game = state.games.lock().unwrap().iter().find(|g| g.title_id == info.title_id && g.platform == "ps4" && !g.archive).map(|g| PathBuf::from(&g.path));
    let dest = pkginstall::destination(&info, &library, game.as_deref(), &shadps4_user(&state));
    Ok(PkgPlan { exists: dest.exists(), destination: dest.to_string_lossy().into_owned(), game_found: game.is_some(), info })
}

#[tauri::command]
async fn pkg_install(app: AppHandle, state: State<'_, AppState>, path: String, folder: Option<String>, overwrite: bool) -> Result<String, String> {
    if !PS4_ENABLED {
        return Err("Installing .pkg files comes in a later version.".into());
    }
    let plan = pkg_inspect(state.clone(), path.clone(), folder).await?;
    let dest = PathBuf::from(&plan.destination);
    logs::info("pkg", &format!("Installing {} ({} {}) to {}", path, plan.info.kind_name, plan.info.title_id, dest.display()));
    let app2 = app.clone();
    let src = PathBuf::from(&path);
    let res = tauri::async_runtime::spawn_blocking(move || {
        pkginstall::install(&src, &dest, overwrite, &|p| {
            let _ = app2.emit("pkg-progress", &p);
        })
    })
    .await
    .map_err(|e| e.to_string())?;
    let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match res {
        Ok(d) => {
            logs::info("pkg", &format!("Installed to {}", d.display()));
            let _ = app.emit("pkg-progress", pkginstall::Progress { file: name, stage: "done".into(), files_done: 0, files_total: 0, bytes_done: 0, bytes_total: 0, message: d.to_string_lossy().into_owned() });
            Ok(d.to_string_lossy().into_owned())
        }
        Err(e) => {
            if !e.starts_with("already-installed:") {
                logs::error("pkg", &e);
                let _ = app.emit("pkg-progress", pkginstall::Progress { file: name, stage: "error".into(), files_done: 0, files_total: 0, bytes_done: 0, bytes_total: 0, message: e.clone() });
            }
            Err(e)
        }
    }
}

#[tauri::command]
fn game_extras(state: State<AppState>, id: String) -> Result<pkginstall::Extras, String> {
    let g = state.games.lock().unwrap().iter().find(|g| g.id == id).cloned().ok_or("Game not found.")?;
    if g.archive {
        return Ok(pkginstall::Extras::default());
    }
    Ok(pkginstall::extras(std::path::Path::new(&g.path), &g.title_id, &shadps4_user(&state)))
}

#[tauri::command]
fn set_mods(state: State<AppState>, id: String, enabled: bool) -> Result<String, String> {
    let g = state.games.lock().unwrap().iter().find(|g| g.id == id).cloned().ok_or("Game not found.")?;
    let p = pkginstall::set_mods(std::path::Path::new(&g.path), enabled)?;
    logs::info("mods", &format!("Mods for {} {}", g.title, if enabled { "on" } else { "off" }));
    Ok(p.to_string_lossy().into_owned())
}

#[tauri::command]
fn skipped_folders(state: State<AppState>) -> Vec<games::Skipped> {
    state.skipped.lock().unwrap().clone()
}

#[tauri::command]
async fn game_size(path: String) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = std::path::Path::new(&path);
        if p.is_file() {
            p.metadata().map(|m| m.len() / 1_048_576).unwrap_or(0)
        } else {
            games::scan(&[path.clone()], true).first().map(|g| g.size_mb).unwrap_or(0)
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().blocking_pick_folder().map(|p| p.to_string())
}

#[tauri::command]
async fn pick_file(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let exts: &[&str] = if cfg!(windows) { &["exe"] } else { &[""] };
    app.dialog().file().add_filter("Engine", exts).blocking_pick_file().map(|p| p.to_string())
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("{path} does not exist"));
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&p).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(&p).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(&p).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

/// Opens a web page in the default browser (only Strata's own links: https on known sites).
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    const OK: [&str; 6] = ["https://ko-fi.com/", "https://github.com/", "https://www.youtube.com/", "https://youtube.com/", "https://strata.gbxstudios.com", "https://www.paypal.com/"];
    if !OK.iter().any(|p| url.starts_with(p)) || url.contains(['"', ' ', '\n']) {
        return Err("This link is not allowed.".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&url).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

#[derive(Clone, Serialize)]
struct GameExit {
    game_id: String,
    code: Option<i32>,
    seconds: u64,
    message: String,
}

#[derive(Clone, Serialize)]
struct EngineEvent {
    game_id: String,
    message: String,
}

/// Everything one launch needs, carried from one engine attempt to the next.
#[derive(Clone)]
struct Attempt {
    game: games::Game,
    chain: Vec<String>,
    idx: usize,
    emu: settings::EmuSettings,
    perf: settings::PerfSettings,
    dev: settings::DevSettings,
    controls: settings::ControlSettings,
    fail_seconds: u32,
    failures: Vec<String>,
    patch_file: String,
    /// Engine release per chain entry, for the compatibility record.
    releases: Vec<String>,
}

fn compat_key(g: &games::Game) -> String {
    if g.title_id.is_empty() { engines::safe_key(&g.id) } else { g.title_id.clone() }
}

/// Release folder of the installed engine ("" if missing): changes when the engine updates.
fn engine_release(s: &Settings, data: &std::path::Path, id: &str) -> String {
    engines::def(id)
        .and_then(|d| engines::find(d, &s.engine_path(id), data))
        .and_then(|p| p.parent().and_then(|x| x.file_name()).map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

/// The game's record with an up-to-date analysis of its eboot.bin (reads the file if needed).
fn ensure_record(data: &std::path::Path, g: &games::Game) -> compat::GameRecord {
    let key = compat_key(g);
    let mut rec = compat::load(data, &key);
    if !g.archive && !compat::is_fresh(&rec.analysis, std::path::Path::new(&g.path)) {
        rec.analysis = compat::analyze(std::path::Path::new(&g.path));
        rec.title = g.title.clone();
        rec.title_id = g.title_id.clone();
        let _ = compat::save(data, &key, &rec);
        logs::debug("compat", &format!("Analyzed {}: {} functions, {} libraries, engine {}", g.title, rec.analysis.imports.len(), rec.analysis.libraries.len(), if rec.analysis.game_engine.is_empty() { "?" } else { &rec.analysis.game_engine }));
    }
    rec
}

fn engine_scores(s: &Settings, data: &std::path::Path, g: &games::Game, rec: &compat::GameRecord, k: &compat::EngineKnowledge) -> Vec<compat::Score> {
    let mut k = k.clone();
    let others = if s.community.use_results {
        let ck = community::load(data);
        community::merge_into(&mut k, &ck);
        community::worked_for_others(&ck, &g.title_id)
    } else {
        vec![]
    };
    engines::ENGINES
        .iter()
        .filter(|d| engines::supports(d, g) && (PS4_ENABLED || d.platforms.contains(&engines::Platform::Ps5)))
        .map(|d| {
            let n = others.iter().find(|(e, _)| e == d.id).map(|(_, n)| *n).unwrap_or(0);
            compat::score(d.id, &engine_release(s, data, d.id), rec, &k, n)
        })
        .collect()
}

fn engine_name(id: &str) -> &'static str {
    engines::def(id).map(|d| d.name).unwrap_or("engine")
}

fn installed_fn(s: &Settings, data: &std::path::Path) -> impl Fn(&str) -> bool {
    let paths = s.engines.paths.clone();
    let data = data.to_path_buf();
    move |id: &str| engines::def(id).is_some_and(|d| engines::find(d, paths.get(id).map(String::as_str).unwrap_or(""), &data).is_some())
}

/// Refresh rate of the monitor Strata's window is on.
fn monitor_hz(app: &AppHandle) -> Option<u32> {
    let w = app.get_webview_window("main")?;
    let m = w.current_monitor().ok()??;
    display::refresh_rate(m.name()?)
}

#[tauri::command]
fn monitor_refresh(app: AppHandle) -> Option<u32> {
    monitor_hz(&app)
}

#[tauri::command]
async fn launch_game(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<engine::RunningInfo, String> {
    if state.running.lock().unwrap().is_some() {
        return Err("A game is already running. Stop it first.".into());
    }
    let game = state
        .games
        .lock()
        .unwrap()
        .iter()
        .find(|g| g.id == id)
        .cloned()
        .ok_or("Game not found — refresh the library.")?;
    let mut attempt = {
        let s = state.settings.lock().unwrap();
        let entry = s.games.get(&id).cloned().unwrap_or_default();
        let platform = engines::Platform::parse(&game.platform);
        let order = if platform == engines::Platform::Ps4 { &s.engines.order_ps4 } else { &s.engines.order_ps5 };
        let installed = installed_fn(&s, &state.data);
        // Full list first; it is sorted by known results below and cut to one if fallback is off.
        let chain = engines::chain(&game, order, &entry.engine, &entry.good_engine, true, &installed);
        if chain.is_empty() {
            // The UI turns this into a friendly prompt with a Download button.
            return Err(format!("need-engine:{}", platform.as_str()));
        }
        Attempt {
            game: game.clone(),
            chain,
            idx: 0,
            emu: s.emu_for(&id),
            perf: s.perf_for(&id),
            dev: s.dev.clone(),
            controls: s.controls_for(&id),
            fail_seconds: s.engines.fail_seconds,
            failures: vec![],
            patch_file: entry.patch_file.clone(),
            releases: vec![],
        }
    };
    // Order the engines by what is known about this game (2nd launch: straight to the one that worked).
    {
        let forced = state.settings.lock().unwrap().games.get(&id).map(|e| !e.engine.is_empty()).unwrap_or(false);
        let (s, data, g) = (state.settings.lock().unwrap().clone(), state.data.clone(), game.clone());
        let chain = attempt.chain.clone();
        let (ordered, releases) = tauri::async_runtime::spawn_blocking(move || {
            let rec = ensure_record(&data, &g);
            let k = compat::load_knowledge(&data);
            let scores = engine_scores(&s, &data, &g, &rec, &k);
            let mut chain = chain;
            if !forced {
                compat::order(&mut chain, &scores);
            }
            for sc in &scores {
                logs::debug("compat", &format!("{}: {} ({}) — {}", engine_name(&sc.engine), sc.score, sc.label, sc.reason));
            }
            let releases = chain.iter().map(|e| engine_release(&s, &data, e)).collect::<Vec<_>>();
            (chain, releases)
        })
        .await
        .map_err(|e| e.to_string())?;
        attempt.chain = ordered;
        attempt.releases = releases;
        if !state.settings.lock().unwrap().engines.auto_fallback {
            attempt.chain.truncate(1);
            attempt.releases.truncate(1);
        }
    }
    // "Automatic" on a PC with two GPUs: the engine may pick the integrated one, so pin the best one.
    if attempt.emu.gpu_index < 0 {
        let cached = state.sys.lock().unwrap().clone();
        let sys = match cached {
            Some(s) => s,
            None => {
                let s = tauri::async_runtime::spawn_blocking(gpu::system_info).await.map_err(|e| e.to_string())?;
                *state.sys.lock().unwrap() = Some(s.clone());
                s
            }
        };
        if sys.gpus.len() > 1 {
            if let Some(best) = gpu::best_gpu(&sys.gpus) {
                attempt.emu.gpu_index = best.index;
                logs::info("engine", &format!("Automatic GPU: using #{} {}", best.index, best.name));
            }
        }
    }
    // Virtual refresh rate "Auto": match the monitor Strata is on.
    if attempt.emu.vblank_frequency == 0 {
        let hz = monitor_hz(&app);
        attempt.emu.vblank_frequency = display::resolve_vblank(0, hz);
        logs::info("engine", &format!("Automatic refresh rate: {} Hz{}", attempt.emu.vblank_frequency, if hz.is_none() { " (monitor not detected)" } else { "" }));
    }
    logs::info("engine", &format!("{} ({}) — engines to try: {}", game.title, game.platform.to_uppercase(), attempt.chain.iter().map(|i| engine_name(i)).collect::<Vec<_>>().join(" → ")));
    {
        let mut s = state.settings.lock().unwrap();
        let entry = s.games.entry(id.clone()).or_default();
        entry.launches += 1;
        entry.last_played = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0));
        let _ = state.save(&s);
    }
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || start_attempt(app2, attempt)).await.map_err(|e| e.to_string())?
}

/// Tries the engines of the chain from `a.idx` until one starts. Blocking.
fn start_attempt(app: AppHandle, mut a: Attempt) -> Result<engine::RunningInfo, String> {
    let state = app.state::<AppState>();
    while a.idx < a.chain.len() {
        let id = a.chain[a.idx].clone();
        let Some(def) = engines::def(&id) else {
            a.idx += 1;
            continue;
        };
        let configured = state.settings.lock().unwrap().engine_path(&id);
        let Some(exe) = engines::find(def, &configured, &state.data) else {
            a.failures.push(format!("{}: not installed", def.name));
            a.idx += 1;
            continue;
        };
        if def.kind == engines::Kind::ShadPs4 {
            // shadPS4 only uses "<working folder>/user" when that folder already exists; otherwise it
            // falls back to %APPDATA%\shadPS4 and ignores Strata's controls, DLC and modules.
            let user = engines::root_dir(&state.data, "shadps4").join("user");
            let _ = std::fs::create_dir_all(user.join("sys_modules"));
        }
        let key = if a.game.title_id.is_empty() { engines::safe_key(&a.game.id) } else { a.game.title_id.clone() };
        let controls_file = match controls::write_for(def.kind, &a.controls, &state.data, &a.game.title_id, &key) {
            Ok(f) => f,
            Err(e) => {
                logs::warn("controls", &format!("Could not write controls for {}: {e}", def.name));
                None
            }
        };
        let plan = engines::plan(def, engines::PlanInput { exe: &exe, game: &a.game, emu: &a.emu, dev: &a.dev, data: &state.data, controls_file, patch_file: &a.patch_file });
        if let Some(conv) = &plan.convert {
            let _ = app.emit("engine-status", EngineEvent { game_id: a.game.id.clone(), message: format!("Preparing {} for {} (first launch only)…", a.game.title, def.name) });
            if let Err(e) = engine::convert(&exe, conv, &plan.program, std::path::Path::new(&engines::eboot(&a.game))) {
                logs::error("anyps5", &e);
                a.failures.push(format!("{}: {e}", def.name));
                a.idx += 1;
                continue;
            }
        }
        let on_exit = {
            let app = app.clone();
            let a = a.clone();
            move |x: engine::ExitInfo| on_engine_exit(app, a, x)
        };
        match engine::launch(
            engine::LaunchRequest {
                plan: &plan,
                engine_id: def.id,
                engine_name: def.name,
                attempt: a.idx + 1,
                attempts: a.chain.len(),
                game_id: &a.game.id,
                title: &a.game.title,
                perf: &a.perf,
                data: &state.data,
            },
            on_exit,
        ) {
            Ok(r) => {
                let info = r.info.clone();
                // Hold the slot while checking, so an exit right now clears it after we fill it.
                let mut slot = state.running.lock().unwrap();
                let exited_already = r.child.lock().unwrap().try_wait().ok().flatten().is_some();
                if !exited_already {
                    *slot = Some(r);
                    let _ = app.emit("game-start", &info);
                    if !state.settings.lock().unwrap().community.discord {
                        logs::debug("discord", "Show on Discord is off (Settings → Community)");
                    }
                    if state.settings.lock().unwrap().community.discord {
                        discord::playing(a.game.title.clone(), def.name.to_string(), a.game.platform.clone(), (info.started_ms / 1000) as i64);
                    }
                }
                return Ok(info);
            }
            Err(e) => {
                logs::error("engine", &e);
                a.failures.push(format!("{}: {e}", def.name));
                a.idx += 1;
            }
        }
    }
    let tried = if a.failures.is_empty() { String::new() } else { format!(" Tried: {}.", a.failures.join("; ")) };
    Err(format!("No engine could start {}.{tried}", a.game.title))
}

fn on_engine_exit(app: AppHandle, a: Attempt, x: engine::ExitInfo) {
    let state = app.state::<AppState>();
    let id = a.chain[a.idx].clone();
    let name = engine_name(&id);
    let mut message = engine::explain_exit_with(x.code, &x.findings.reason);
    if x.code.unwrap_or(0) == 0 {
        logs::info("engine", &format!("{name}: {message}"));
    } else {
        logs::error("engine", &format!("{name}: {message}"));
    }
    let failed = engines::attempt_failed(x.code, x.seconds, a.fail_seconds, x.stopped, x.skip);
    // Remember what happened (and what was missing) for next time and for other games.
    let worked = !failed && x.seconds >= a.fail_seconds as u64;
    if failed || worked {
        let release = a.releases.get(a.idx).cloned().unwrap_or_default();
        let key = compat_key(&a.game);
        let mut rec = compat::load(&state.data, &key);
        rec.title = a.game.title.clone();
        rec.title_id = a.game.title_id.clone();
        let attempt = compat::Attempt {
            engine: id.clone(),
            release: release.clone(),
            when: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            seconds: x.seconds,
            code: x.code,
            worked,
            findings: x.findings.clone(),
        };
        compat::add_attempt(&mut rec, attempt.clone());
        let _ = compat::save(&state.data, &key, &rec);
        // Opt-in: share the anonymous result (background thread, never blocks).
        if state.settings.lock().unwrap().community.share && !x.skip {
            let gpu = state.sys.lock().unwrap().as_ref().and_then(|s| gpu::best_gpu(&s.gpus).map(|g| g.name.clone())).unwrap_or_default();
            if let Some(r) = community::build(&a.game, &rec, &attempt, &gpu) {
                community::send(r);
            }
        }
        if failed && !x.skip {
            let mut k = compat::load_knowledge(&state.data);
            compat::learn(&mut k, &id, &release, &x.findings);
            let _ = compat::save_knowledge(&state.data, &k);
            if !x.findings.missing_functions.is_empty() || !x.findings.missing_modules.is_empty() {
                logs::info("compat", &format!("{name} is missing {} function(s) and {} module(s) for {}", x.findings.missing_functions.len(), x.findings.missing_modules.len(), a.game.title));
            }
        }
    }
    {
        let mut s = state.settings.lock().unwrap();
        let keep = s.dev.keep_sessions.max(1) as usize;
        let entry = s.games.entry(x.game_id.clone()).or_default();
        entry.play_seconds += x.seconds;
        if !failed && x.seconds >= a.fail_seconds as u64 {
            entry.good_engine = id.clone();
        }
        let _ = s.save(&state.data.join("settings.json"));
        logs::prune_sessions(&state.data, keep);
    }
    {
        let mut slot = state.running.lock().unwrap();
        if slot.as_ref().is_some_and(|r| r.info.game_id == x.game_id) {
            *slot = None;
        }
    }
    if failed && a.idx + 1 < a.chain.len() {
        let next_name = engine_name(&a.chain[a.idx + 1]);
        let why = if x.skip {
            "skipped".to_string()
        } else if x.findings.reason.is_empty() {
            format!("closed after {} s", x.seconds)
        } else {
            let r: String = x.findings.reason.chars().take(90).collect();
            format!("closed after {} s ({r})", x.seconds)
        };
        let msg = format!("{name} {why} — trying {next_name}…");
        logs::warn("engine", &msg);
        let _ = app.emit("engine-fallback", EngineEvent { game_id: x.game_id.clone(), message: msg });
        let mut next = a.clone();
        next.failures.push(format!("{name}: {why}"));
        next.idx += 1;
        if let Err(e) = start_attempt(app.clone(), next) {
            logs::error("engine", &e);
            message = e;
        } else {
            return;
        }
    } else if failed && a.chain.len() > 1 && !x.stopped {
        message = format!("{message} No other engine is left to try.");
    }
    if state.settings.lock().unwrap().community.discord {
        discord::idle(state.games.lock().unwrap().len(), now_unix());
    } else {
        discord::clear();
    }
    let _ = app.emit("game-exit", GameExit { game_id: x.game_id, code: x.code, seconds: x.seconds, message });
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn stop_game(state: State<AppState>) -> Result<(), String> {
    let guard = state.running.lock().unwrap();
    let Some(r) = guard.as_ref() else { return Ok(()) };
    logs::info("engine", &format!("Stopping {} (ran {} s)", r.info.title, r.started.elapsed().as_secs()));
    r.stopped.store(true, std::sync::atomic::Ordering::SeqCst);
    let res = r.child.lock().unwrap().kill().map_err(|e| e.to_string());
    res
}

/// Closes the current engine and starts the next one in the chain.
#[tauri::command]
fn next_engine(state: State<AppState>) -> Result<(), String> {
    let guard = state.running.lock().unwrap();
    let Some(r) = guard.as_ref() else { return Ok(()) };
    if r.info.attempt >= r.info.attempts {
        return Err("This was the last engine for this game.".into());
    }
    logs::info("engine", &format!("Skipping {} for {}", r.info.engine_name, r.info.title));
    r.skip.store(true, std::sync::atomic::Ordering::SeqCst);
    let res = r.child.lock().unwrap().kill().map_err(|e| e.to_string());
    res
}

#[tauri::command]
fn running_game(state: State<AppState>) -> Option<engine::RunningInfo> {
    state.running.lock().unwrap().as_ref().map(|r| r.info.clone())
}

#[tauri::command]
fn logs_since(seq: u64) -> Vec<logs::LogLine> {
    logs::since(seq)
}

#[tauri::command]
fn clear_logs() {
    logs::clear();
}

#[tauri::command]
fn ui_log(level: String, msg: String) {
    let lvl: &'static str = match level.as_str() {
        "error" => "error",
        "warn" => "warn",
        "debug" => "debug",
        _ => "info",
    };
    logs::log(lvl, "ui", &msg);
}

#[tauri::command]
async fn run_selftest(state: State<'_, AppState>) -> Result<Vec<selftest::Check>, String> {
    let s = state.settings.lock().unwrap().clone();
    let data = state.data.clone();
    let checks = tauri::async_runtime::spawn_blocking(move || selftest::run_all(&s, &data)).await.map_err(|e| e.to_string())?;
    for c in &checks {
        let lvl = match c.status.as_str() {
            "fail" => "error",
            "warn" => "warn",
            _ => "info",
        };
        logs::log(lvl, "selftest", &format!("{}: {} ({} ms)", c.name, c.detail, c.ms));
    }
    Ok(checks)
}

#[derive(Serialize)]
struct EngineInfo {
    path: Option<String>,
    version: Option<String>,
    error: Option<String>,
}

/// KytyPS5 details (it answers --help with its version).
#[tauri::command]
async fn engine_info(state: State<'_, AppState>) -> Result<EngineInfo, String> {
    let (configured, data) = (state.settings.lock().unwrap().engine_path("kyty"), state.data.clone());
    tauri::async_runtime::spawn_blocking(move || match engines::find(engines::def("kyty").unwrap(), &configured, &data) {
        None => EngineInfo { path: None, version: None, error: Some("not installed".into()) },
        Some(p) => {
            let path = Some(p.to_string_lossy().into_owned());
            match engine::engine_version(&p) {
                Ok(v) => EngineInfo { path, version: Some(v), error: None },
                Err(e) => EngineInfo { path, version: None, error: Some(e) },
            }
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct EngineStatus {
    id: &'static str,
    name: &'static str,
    repo: &'static str,
    summary: &'static str,
    platforms: Vec<&'static str>,
    archives: bool,
    path: Option<String>,
    /// Release folder name (engines are never started just to ask their version).
    release: Option<String>,
    controls: bool,
}

#[tauri::command]
fn engines_status(state: State<AppState>) -> Vec<EngineStatus> {
    let s = state.settings.lock().unwrap();
    engines::ENGINES
        .iter()
        .filter(|d| PS4_ENABLED || d.platforms.contains(&engines::Platform::Ps5))
        .map(|d| {
            let p = engines::find(d, &s.engine_path(d.id), &state.data);
            EngineStatus {
                id: d.id,
                name: d.name,
                repo: d.repo,
                summary: d.summary,
                platforms: d.platforms.iter().filter(|p| PS4_ENABLED || **p == engines::Platform::Ps5).map(|p| p.as_str()).collect(),
                archives: d.archives,
                release: p.as_ref().and_then(|x| x.parent()).and_then(|x| x.file_name()).map(|n| n.to_string_lossy().into_owned()),
                path: p.map(|x| x.to_string_lossy().into_owned()),
                controls: matches!(d.kind, engines::Kind::ShadPs4 | engines::Kind::AnyPs5),
            }
        })
        .collect()
}

#[tauri::command]
async fn download_engine(app: AppHandle, state: State<'_, AppState>, id: Option<String>) -> Result<String, String> {
    let id = id.unwrap_or_else(|| "kyty".into());
    let def = engines::def(&id).filter(|d| PS4_ENABLED || d.platforms.contains(&engines::Platform::Ps5)).ok_or("Unknown engine.")?;
    let data = state.data.clone();
    let app2 = app.clone();
    let dest = tauri::async_runtime::spawn_blocking(move || {
        download::download_latest(def, &data, &|p| {
            let _ = app2.emit("engine-download", &p);
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| {
        logs::error("download", &format!("{}: {e}", def.name));
        let _ = app.emit("engine-download", download::Progress { stage: "error".into(), received: 0, total: 0, message: e.clone(), engine: def.id.into() });
        e
    })?;
    let exe = engines::find(def, &dest.to_string_lossy(), &state.data).ok_or(format!("Downloaded, but {} was not inside the release.", def.name))?;
    let mut s = state.settings.lock().unwrap();
    // The fresh download wins over an older folder chosen by hand.
    s.engines.paths.insert(id.clone(), exe.to_string_lossy().into_owned());
    if id == "kyty" {
        s.engine_path = exe.to_string_lossy().into_owned();
    }
    if engines::def(&id).is_some_and(|d| d.platforms.contains(&engines::Platform::Ps4)) && id == "shadps4" {
        s.engines.skip_ps4 = false;
    }
    state.save(&s)?;
    logs::info("download", &format!("{} ready: {}", def.name, exe.display()));
    Ok(exe.to_string_lossy().into_owned())
}

#[derive(Serialize)]
struct ControlInfo {
    actions: Vec<(&'static str, &'static str)>,
    keys: Vec<String>,
}

#[tauri::command]
fn control_info() -> ControlInfo {
    let mut keys: Vec<String> = ('a'..='z').map(String::from).collect();
    keys.extend(('0'..='9').map(String::from));
    keys.extend((1..=12).map(|n| format!("f{n}")));
    keys.extend((0..=9).map(|n| format!("kp{n}")));
    keys.extend(controls::KEYS.iter().map(|k| k.0.to_string()));
    ControlInfo { actions: controls::ACTIONS.iter().map(|(a, l, _)| (*a, *l)).collect(), keys }
}

fn backup_roots(state: &AppState) -> Vec<(String, PathBuf)> {
    let s = state.settings.lock().unwrap();
    engines::ENGINES
        .iter()
        .flat_map(|d| {
            let exe = engines::find(d, &s.engine_path(d.id), &state.data);
            engines::user_dirs(d, &state.data, exe.as_deref()).into_iter().map(move |p| (d.id.to_string(), p))
        })
        .collect()
}

#[tauri::command]
async fn backup_create(state: State<'_, AppState>, game_id: Option<String>, shaders: bool) -> Result<String, String> {
    let game = game_id.and_then(|id| state.games.lock().unwrap().iter().find(|g| g.id == id).cloned());
    let roots = backup_roots(&state);
    let data = state.data.clone();
    let (label, title_id) = game.map(|g| (g.title, g.title_id)).unwrap_or(("All games".into(), String::new()));
    let res = tauri::async_runtime::spawn_blocking(move || {
        let sources = backups::find_sources(&roots, &title_id, shaders);
        backups::create(&data, &label, &title_id, &sources, &logs::stamp())
    })
    .await
    .map_err(|e| e.to_string())?;
    let (path, files) = res?;
    logs::info("backup", &format!("Saved {files} file(s) to {}", path.display()));
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn backup_list(state: State<AppState>) -> Vec<backups::BackupInfo> {
    backups::list(&state.data)
}

#[tauri::command]
async fn backup_restore(state: State<'_, AppState>, file: String) -> Result<usize, String> {
    if state.running.lock().unwrap().is_some() {
        return Err("Close the game before restoring a backup.".into());
    }
    let n = tauri::async_runtime::spawn_blocking(move || backups::restore(std::path::Path::new(&file))).await.map_err(|e| e.to_string())??;
    logs::info("backup", &format!("Restored {n} file(s)"));
    Ok(n)
}

#[tauri::command]
async fn export_diagnostics(state: State<'_, AppState>) -> Result<String, String> {
    let checks = run_selftest(state.clone()).await?;
    let sys = system_info(state.clone(), false).await?;
    let s = state.settings.lock().unwrap().clone();
    let mut out = String::new();
    out.push_str(&format!("{APP_NAME} {APP_VERSION} diagnostics — {}\n\n", logs::stamp()));
    out.push_str(&format!("[system]\n{}\n\n", serde_json::to_string_pretty(&sys).unwrap_or_default()));
    out.push_str("[self-test]\n");
    for c in &checks {
        out.push_str(&format!("{:4} {:18} {}\n", c.status, c.name, c.detail));
    }
    let mut redacted = s.clone();
    redacted.emu.user_name = "<hidden>".into();
    out.push_str(&format!("\n[settings]\n{}\n\n[recent log]\n", serde_json::to_string_pretty(&redacted).unwrap_or_default()));
    let lines = logs::since(0);
    for l in lines.iter().rev().take(800).collect::<Vec<_>>().into_iter().rev() {
        out.push_str(&format!("{} {:5} [{}] {}\n", l.t, l.level, l.source, l.msg));
    }
    let path = logs::logs_dir(&state.data).join(format!("diagnostics-{}.txt", logs::stamp()));
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    logs::info("dev", &format!("Diagnostics saved to {}", path.display()));
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn paths(state: State<AppState>) -> serde_json::Value {
    serde_json::json!({
        "data": state.data,
        "logs": logs::logs_dir(&state.data),
        "games": state.default_games_dir(),
        "engine": state.data.join("engine"),
        "engines": state.data.join("engines"),
        "backups": backups::backups_dir(&state.data),
        "compat": compat::dir(&state.data),
        "shadps4_modules": engines::root_dir(&state.data, "shadps4").join("user").join("sys_modules"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_go_to_the_games_drive() {
        let root = std::env::temp_dir().join(format!("strata_target_{}", std::process::id()));
        let own = root.join("data/Games");
        let hd = root.join("E/PS5Games");
        std::fs::create_dir_all(&own).unwrap();
        std::fs::create_dir_all(&hd).unwrap();
        let mut s = Settings::default();
        s.library_folders = vec![own.to_string_lossy().into(), hd.to_string_lossy().into()];
        assert_eq!(pkg_target(&s, &own), hd, "the user's own folder beats Strata's");
        s.library_folders = vec![own.to_string_lossy().into()];
        assert_eq!(pkg_target(&s, &own), own);
        s.pkg_install_folder = hd.to_string_lossy().into();
        assert_eq!(pkg_target(&s, &own), hd, "a chosen folder wins");
        s.pkg_install_folder = root.join("missing").to_string_lossy().into();
        assert_eq!(pkg_target(&s, &own), own, "a missing chosen folder is ignored");
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    STARTED_UNIX.get_or_init(now_unix);
    let data = settings::data_dir();
    let _ = std::fs::create_dir_all(&data);
    let settings = Settings::load(&data.join("settings.json"));
    logs::init(&data, None, &settings.dev.log_level);
    logs::info("app", &format!("{APP_NAME} {APP_VERSION} {APP_CHANNEL} starting · data: {}", data.display()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            data: data.clone(),
            settings: Arc::new(Mutex::new(settings.clone())),
            running: Arc::new(Mutex::new(None)),
            sys: Mutex::new(None),
            games: Mutex::new(Vec::new()),
            skipped: Mutex::new(Vec::new()),
            pkgs: Mutex::new(Vec::new()),
        })
        .setup(move |app| {
            logs::init(&data, Some(app.handle().clone()), &settings.dev.log_level);
            if settings.community.use_results {
                community::refresh(data.clone());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            save_settings,
            system_info,
            auto_profile,
            apply_auto_profile,
            scan_games,
            game_size,
            pick_folder,
            pick_file,
            open_path,
            open_url,
            launch_game,
            stop_game,
            next_engine,
            running_game,
            monitor_refresh,
            skipped_folders,
            found_pkgs,
            game_compat,
            library_states,
            stats,
            discord_available,
            check_update,
            compat_reset,
            pick_pkg,
            pick_patch,
            pkg_inspect,
            pkg_target_folder,
            pkg_install,
            game_extras,
            set_mods,
            engines_status,
            control_info,
            backup_create,
            backup_list,
            backup_restore,
            logs_since,
            clear_logs,
            ui_log,
            run_selftest,
            engine_info,
            download_engine,
            export_diagnostics,
            paths
        ])
        .run(tauri::generate_context!())
        .expect("error while running Strata");
}
