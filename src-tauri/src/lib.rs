//! Strata — a launcher frontend. The UI lives in ../ui; this side talks to the OS and the engine.

pub mod download;
pub mod engine;
pub mod games;
pub mod gpu;
pub mod logs;
pub mod memcap;
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

pub struct AppState {
    pub data: PathBuf,
    pub settings: Arc<Mutex<Settings>>,
    pub running: Arc<Mutex<Option<engine::Running>>>,
    pub sys: Mutex<Option<gpu::SystemInfo>>,
    pub games: Mutex<Vec<games::Game>>,
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
    let engine = engine::find_engine(&s.engine_path, &state.data).map(|p| p.to_string_lossy().into_owned());
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
    }
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    let mut new = settings;
    new.sanitize();
    logs::set_level(&new.dev.log_level);
    state.save(&new)?;
    *state.settings.lock().unwrap() = new.clone();
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
async fn scan_games(state: State<'_, AppState>) -> Result<Vec<games::Game>, String> {
    let folders = state.settings.lock().unwrap().library_folders.clone();
    let t = std::time::Instant::now();
    let list = tauri::async_runtime::spawn_blocking(move || games::scan(&folders, false)).await.map_err(|e| e.to_string())?;
    logs::info("library", &format!("{} game(s) found in {} ms", list.len(), t.elapsed().as_millis()));
    *state.games.lock().unwrap() = list.clone();
    Ok(list)
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

#[derive(Clone, Serialize)]
struct GameExit {
    game_id: String,
    code: Option<i32>,
    seconds: u64,
    message: String,
}

#[tauri::command]
fn launch_game(app: AppHandle, state: State<AppState>, id: String) -> Result<engine::RunningInfo, String> {
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
    let (engine_path, emu, perf, dev) = {
        let s = state.settings.lock().unwrap();
        let e = engine::find_engine(&s.engine_path, &state.data)
            .ok_or("Engine not found. Open Settings → Engine to download it or choose its folder.")?;
        (e, s.emu_for(&id), s.perf_for(&id), s.dev.clone())
    };
    // "Automatic" on a PC with two GPUs: the engine may pick the integrated one, so pin the best one.
    let mut emu = emu;
    if emu.gpu_index < 0 {
        let cached = state.sys.lock().unwrap().clone();
        let sys = cached.unwrap_or_else(|| {
            let s = gpu::system_info();
            *state.sys.lock().unwrap() = Some(s.clone());
            s
        });
        if sys.gpus.len() > 1 {
            if let Some(best) = gpu::best_gpu(&sys.gpus) {
                emu.gpu_index = best.index;
                logs::info("engine", &format!("Automatic GPU: using #{} {}", best.index, best.name));
            }
        }
    }
    let target = engine::game_target(&game.path, game.archive);

    let settings = state.settings.clone();
    let running = state.running.clone();
    let data = state.data.clone();
    let app2 = app.clone();
    let on_exit = move |x: engine::ExitInfo| {
        let message = engine::explain_exit(x.code);
        if x.code.unwrap_or(0) == 0 {
            logs::info("engine", &message);
        } else {
            logs::error("engine", &message);
        }
        {
            let mut s = settings.lock().unwrap();
            let entry = s.games.entry(x.game_id.clone()).or_default();
            entry.play_seconds += x.seconds;
            let _ = s.save(&data.join("settings.json"));
            logs::prune_sessions(&data, s.dev.keep_sessions.max(1) as usize);
        }
        {
            // Only clear the slot if it still belongs to this game (a new one may have started).
            let mut slot = running.lock().unwrap();
            if slot.as_ref().is_some_and(|r| r.info.game_id == x.game_id) {
                *slot = None;
            }
        }
        let _ = app2.emit("game-exit", GameExit { game_id: x.game_id, code: x.code, seconds: x.seconds, message });
        if let Some(w) = app2.get_webview_window("main") {
            let _ = w.unminimize();
            let _ = w.set_focus();
        }
    };

    let r = engine::launch(
        engine::LaunchRequest {
            engine: &engine_path,
            game_id: &id,
            title: &game.title,
            target: &target,
            emu: &emu,
            perf: &perf,
            dev: &dev,
            data: &state.data,
        },
        on_exit,
    )?;
    let info = r.info.clone();
    {
        let mut s = state.settings.lock().unwrap();
        let entry = s.games.entry(id.clone()).or_default();
        entry.launches += 1;
        entry.last_played = Some((info.started_ms / 1000) as i64);
        let _ = state.save(&s);
    }
    let exited_already = r.child.lock().unwrap().try_wait().ok().flatten().is_some();
    if !exited_already {
        *state.running.lock().unwrap() = Some(r);
        let _ = app.emit("game-start", &info);
    }
    Ok(info)
}

#[tauri::command]
fn stop_game(state: State<AppState>) -> Result<(), String> {
    let guard = state.running.lock().unwrap();
    let Some(r) = guard.as_ref() else { return Ok(()) };
    logs::info("engine", &format!("Stopping {} (ran {} s)", r.info.title, r.started.elapsed().as_secs()));
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

#[tauri::command]
async fn engine_info(state: State<'_, AppState>) -> Result<EngineInfo, String> {
    let (configured, data) = (state.settings.lock().unwrap().engine_path.clone(), state.data.clone());
    tauri::async_runtime::spawn_blocking(move || match engine::find_engine(&configured, &data) {
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

#[tauri::command]
async fn download_engine(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let data = state.data.clone();
    let app2 = app.clone();
    let dest = tauri::async_runtime::spawn_blocking(move || {
        download::download_latest(&data, &|p| {
            let _ = app2.emit("engine-download", &p);
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| {
        logs::error("download", &e);
        let _ = app.emit("engine-download", download::Progress { stage: "error".into(), received: 0, total: 0, message: e.clone() });
        e
    })?;
    let exe = engine::find_engine(&dest.to_string_lossy(), &state.data).ok_or("Downloaded, but the engine executable was not inside.")?;
    let mut s = state.settings.lock().unwrap();
    s.engine_path = exe.to_string_lossy().into_owned();
    state.save(&s)?;
    Ok(s.engine_path.clone())
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
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        })
        .setup(move |app| {
            logs::init(&data, Some(app.handle().clone()), &settings.dev.log_level);
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
            launch_game,
            stop_game,
            running_game,
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
