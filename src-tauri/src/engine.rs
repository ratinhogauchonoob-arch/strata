//! Running engines: KytyPS5's command line, and starting/watching any engine process.
//! Which engine runs what is decided in engines.rs.

use crate::logs;
use crate::settings::{DevSettings, EmuSettings, PerfSettings};

use serde::Serialize;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(windows)]
pub const ENGINE_EXE: &str = "kyty_emulator.exe";
#[cfg(not(windows))]
pub const ENGINE_EXE: &str = "kyty_emulator";

pub fn parse_resolution(s: &str) -> Option<(u32, u32)> {
    let s = s.trim().to_ascii_lowercase().replace(' ', "");
    let (w, h) = s.split_once(['x', '*'])?;
    let (w, h) = (w.parse::<u32>().ok()?, h.parse::<u32>().ok()?);
    ((320..=16384).contains(&w) && (320..=16384).contains(&h)).then_some((w, h))
}

/// Places that may hold the engine, most specific first.
pub fn candidate_dirs(data: &Path) -> Vec<PathBuf> {
    let mut v = vec![data.join("engine")];
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        v.push(exe_dir.join("engine"));
        v.push(exe_dir.clone());
    }
    v
}

fn search(dir: &Path, depth: usize) -> Option<PathBuf> {
    let direct = dir.join(ENGINE_EXE);
    if direct.is_file() {
        return Some(direct);
    }
    if depth == 0 {
        return None;
    }
    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    subdirs.sort();
    subdirs.into_iter().find_map(|d| search(&d, depth - 1))
}

pub fn find_engine(configured: &str, data: &Path) -> Option<PathBuf> {
    if !configured.trim().is_empty() {
        let p = PathBuf::from(configured.trim().trim_matches('"'));
        if p.is_file() {
            return Some(p);
        }
        if p.is_dir() {
            if let Some(found) = search(&p, 3) {
                return Some(found);
            }
        }
    }
    candidate_dirs(data).iter().find_map(|d| search(d, 3))
}

/// Runs `engine --help` and returns its first line (build/version banner).
pub fn engine_version(engine: &Path) -> Result<String, String> {
    let mut cmd = Command::new(engine);
    cmd.arg("--help").stdout(Stdio::piped()).stderr(Stdio::null()).stdin(Stdio::null());
    if let Some(dir) = engine.parent() {
        cmd.current_dir(dir);
    }
    hide_console(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| format!("cannot start engine: {e}"))?;
    // Read while it runs so a long --help can never fill the pipe and stall the process.
    let reader = child.stdout.take().map(|out| {
        std::thread::spawn(move || {
            let mut first = String::new();
            let mut r = BufReader::new(out);
            let _ = r.read_line(&mut first);
            let _ = std::io::copy(&mut r, &mut std::io::sink());
            first
        })
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > Duration::from_secs(8) => {
                let _ = child.kill();
                return Err("engine did not answer --help within 8 s".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let first = reader.and_then(|h| h.join().ok()).unwrap_or_default();
    let first = first.trim().to_string();
    if first.is_empty() {
        Err("engine printed nothing".into())
    } else {
        Ok(first)
    }
}

fn flag(args: &mut Vec<String>, on: bool, name: &str) {
    if on {
        args.push(name.into());
    }
}

fn opt(args: &mut Vec<String>, name: &str, value: impl ToString) {
    args.push(name.into());
    args.push(value.to_string());
}

/// The game target the engine expects: folder games point at their eboot.bin.
pub fn game_target(path: &str, archive: bool) -> String {
    if archive {
        path.to_string()
    } else {
        Path::new(path).join("eboot.bin").to_string_lossy().into_owned()
    }
}

/// Mirrors the official launcher's argument list so behavior is identical.
pub fn build_args(e: &EmuSettings, dev: &DevSettings, target: &str, logs_dir: &Path) -> Vec<String> {
    let mut a = Vec::new();
    let (w, h) = parse_resolution(&e.resolution).unwrap_or((1280, 720));
    opt(&mut a, "--screen-width", w);
    opt(&mut a, "--screen-height", h);
    opt(&mut a, "--user-name", if e.user_name.trim().is_empty() { "Player" } else { e.user_name.trim() });
    opt(&mut a, "--user-id", e.user_id.max(1));
    if !e.controller_color.is_empty() {
        opt(&mut a, "--controller-color", &e.controller_color);
    }
    opt(&mut a, "--controller-volume", e.speaker_volume.min(100));
    opt(&mut a, "--controller-vibration", e.vibration.min(100));
    opt(&mut a, "--keyboard-player", e.keyboard_player);
    opt(&mut a, "--present-mode", &e.present_mode);
    if e.gpu_index >= 0 {
        opt(&mut a, "--gpu", e.gpu_index);
    }
    flag(&mut a, e.fullscreen, "--fullscreen");
    flag(&mut a, e.hdr, "--hdr");
    flag(&mut a, e.hide_cursor, "--hide-cursor");
    flag(&mut a, e.stretch, "--stretch");
    flag(&mut a, e.readback_linear, "--readback-linear-images");
    flag(&mut a, e.readback_tiled, "--readback-tiled-images");
    flag(&mut a, e.readback_storage, "--readback-storage-images");
    flag(&mut a, e.sync_raw_image_buffers, "--sync-raw-image-buffers");
    opt(&mut a, "--flush-draws", e.flush_draws);
    flag(&mut a, !e.trophies, "--no-trophy-notifications");
    opt(&mut a, "--trophy-duration", e.trophy_duration);
    flag(&mut a, e.perf_overlay, "--perf-overlay");
    flag(&mut a, e.skip_notice_screen, "--skip-notice-screen");
    flag(&mut a, e.network, "--network-connected");
    flag(&mut a, e.emulate_online, "--emulate-psn");
    flag(&mut a, e.tessellation, "--tessellation");
    flag(&mut a, e.occlusion_queries, "--occlusion-queries");
    flag(&mut a, e.relaxed_shader_float, "--relaxed-shader-float");
    flag(&mut a, e.native_rt, "--native-rt");
    flag(&mut a, e.shader_cache, "--shader-cache");
    opt(&mut a, "--vblank-frequency", crate::display::resolve_vblank(e.vblank_frequency, None));
    opt(&mut a, "--console-language", e.console_language);
    flag(&mut a, e.vulkan_validation, "--vulkan-validation");
    flag(&mut a, e.shader_validation, "--shader-validation");
    opt(&mut a, "--shader-log-direction", &e.shader_log);
    opt(&mut a, "--shader-log-folder", logs_dir.join("shaders").to_string_lossy());
    flag(&mut a, e.command_buffer_dump, "--command-buffer-dump");
    opt(&mut a, "--command-buffer-dump-folder", logs_dir.join("buffers").to_string_lossy());
    // The engine's own log goes to stdout so Strata can show and save it.
    opt(&mut a, "--printf-direction", if dev.capture_engine_log { "Console" } else { "Silent" });
    flag(&mut a, e.profiler, "--profile");
    flag(&mut a, e.pro_mode, "--trinity");
    if e.pro_mode {
        opt(&mut a, "--pssr-upscaler", &e.upscaler);
        if e.upscaler == "DLSS" && !e.dlss_folder.is_empty() {
            opt(&mut a, "--dlss-folder", &e.dlss_folder);
        } else if e.upscaler == "FSR" && !e.fsr_folder.is_empty() {
            opt(&mut a, "--fsr-folder", &e.fsr_folder);
        }
    }
    flag(&mut a, e.amd_cpu_compat, "--amd-cpu");
    flag(&mut a, e.guest_tsc, "--guest-tsc");
    if cfg!(windows) {
        flag(&mut a, e.red_zone, "--redzone");
    }
    flag(&mut a, e.renderdoc, "--rd");
    opt(&mut a, "--game", target);
    a
}

/// Quote for display only (the process gets the raw list).
pub fn display_command(engine: &Path, args: &[String]) -> String {
    let q = |s: &str| if s.contains(' ') || s.is_empty() { format!("\"{s}\"") } else { s.to_string() };
    std::iter::once(q(&engine.to_string_lossy())).chain(args.iter().map(|s| q(s))).collect::<Vec<_>>().join(" ")
}

#[cfg(windows)]
fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}
#[cfg(not(windows))]
fn hide_console(_cmd: &mut Command) {}

#[derive(Clone, Debug, Serialize)]
pub struct RunningInfo {
    pub game_id: String,
    pub engine: String,
    pub engine_name: String,
    /// 1-based attempt number and how many engines may be tried.
    pub attempt: usize,
    pub attempts: usize,
    pub title: String,
    pub pid: u32,
    pub started_ms: u64,
    pub command: String,
    pub notes: Vec<String>,
    pub session_log: String,
}

pub struct Running {
    pub info: RunningInfo,
    pub child: Arc<Mutex<Child>>,
    pub started: Instant,
    /// Set by Stop: the exit must not start the next engine.
    pub stopped: Arc<std::sync::atomic::AtomicBool>,
    /// Set by "Try next engine".
    pub skip: Arc<std::sync::atomic::AtomicBool>,
    _guard: crate::memcap::Guard,
}

pub struct ExitInfo {
    pub game_id: String,
    pub code: Option<i32>,
    pub seconds: u64,
    pub stopped: bool,
    pub skip: bool,
    /// What the engine printed that explains a failure (missing functions/modules).
    pub findings: crate::compat::Findings,
}

pub struct LaunchRequest<'a> {
    pub plan: &'a crate::engines::Plan,
    pub engine_id: &'a str,
    pub engine_name: &'a str,
    pub attempt: usize,
    pub attempts: usize,
    pub game_id: &'a str,
    pub title: &'a str,
    pub perf: &'a PerfSettings,
    pub data: &'a Path,
}

/// AnyPS5: converts the game once (cached until the eboot changes). Blocking.
pub fn convert(relinker: &Path, args: &[String], output: &Path, source: &Path) -> Result<bool, String> {
    let fresh = |o: &Path| -> bool {
        let (Ok(om), Ok(sm)) = (o.metadata(), source.metadata()) else { return false };
        matches!((om.modified(), sm.modified()), (Ok(a), Ok(b)) if a >= b)
    };
    if output.is_file() && fresh(output) {
        return Ok(false);
    }
    if let Some(dir) = output.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut cmd = Command::new(relinker);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = relinker.parent() {
        cmd.current_dir(dir);
    }
    hide_console(&mut cmd);
    logs::info("anyps5", "Converting the game (first launch only)…");
    logs::debug("anyps5", &display_command(relinker, args));
    let out = cmd.output().map_err(|e| format!("Could not start the AnyPS5 converter: {e}"))?;
    for line in String::from_utf8_lossy(&out.stdout).lines().chain(String::from_utf8_lossy(&out.stderr).lines()) {
        if !line.trim().is_empty() {
            logs::log(logs::classify(line), "anyps5", line);
        }
    }
    if !out.status.success() || !output.is_file() {
        return Err(format!("AnyPS5 could not convert this game (code {:?}). See Developer → Logs.", out.status.code()));
    }
    Ok(true)
}

/// Starts the game. `on_exit` runs on a background thread when the engine exits.
pub fn launch(req: LaunchRequest, on_exit: impl FnOnce(ExitInfo) + Send + 'static) -> Result<Running, String> {
    let logs_dir = logs::logs_dir(req.data);
    let _ = std::fs::create_dir_all(logs_dir.join("shaders"));
    let _ = std::fs::create_dir_all(logs_dir.join("buffers"));
    let plan = req.plan;
    let args = &plan.args;
    let command = display_command(&plan.program, args);

    let safe_title: String = req.title.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(40).collect();
    let session_path = logs_dir.join(format!("session-{}-{}.log", logs::stamp(), safe_title));
    let session = Arc::new(Mutex::new(std::fs::File::create(&session_path).ok()));
    let findings = Arc::new(Mutex::new(crate::compat::Findings::default()));
    if let Some(f) = session.lock().unwrap().as_mut() {
        let _ = writeln!(
            f,
            "# Strata session log\n# game: {}\n# engine: {} (attempt {} of {})\n# command: {}\n",
            req.title, req.engine_name, req.attempt, req.attempts, command
        );
    }

    let mut cmd = Command::new(&plan.program);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if plan.cwd.is_dir() || std::fs::create_dir_all(&plan.cwd).is_ok() {
        cmd.current_dir(&plan.cwd);
    }
    for (k, v) in &plan.env {
        cmd.env(k, v);
    }
    hide_console(&mut cmd);
    logs::info("engine", &format!("Starting {} with {} ({} of {})", req.title, req.engine_name, req.attempt, req.attempts));
    logs::debug("engine", &command);
    let mut child = cmd.spawn().map_err(|e| format!("Could not start {}: {e}", req.engine_name))?;

    let limit = req.perf.ram_limit_enabled.then(|| (req.perf.ram_limit_gb, crate::memcap::parse_mode(&req.perf.ram_mode)));
    let priority = crate::memcap::resolve_priority_here(&req.perf.priority);
    let (guard, notes) = crate::memcap::apply(&child, limit, &priority);
    for n in &notes {
        logs::info("perf", n);
    }

    for (pipe, is_err) in [(child.stdout.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>), false),
                           (child.stderr.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>), true)]
    {
        let Some(pipe) = pipe else { continue };
        let session = session.clone();
        let findings = findings.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(pipe);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let line = String::from_utf8_lossy(&buf);
                        let line = line.trim_end();
                        if line.is_empty() {
                            continue;
                        }
                        if let Some(f) = session.lock().unwrap().as_mut() {
                            let _ = writeln!(f, "{line}");
                        }
                        crate::compat::scan_line(line, &mut findings.lock().unwrap());
                        let level = if is_err { "warn" } else { logs::classify(line) };
                        let level = if logs::classify(line) == "error" { "error" } else { level };
                        logs::log(level, "engine", line);
                    }
                }
            }
        });
    }

    let pid = child.id();
    let child = Arc::new(Mutex::new(child));
    let started = Instant::now();
    let started_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let skip = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let (stopped, skip) = (stopped.clone(), skip.clone());
        let findings = findings.clone();
        let child = child.clone();
        let game_id = req.game_id.to_string();
        let session = session.clone();
        std::thread::spawn(move || {
            let code = loop {
                let status = child.lock().unwrap().try_wait();
                match status {
                    Ok(Some(s)) => break s.code(),
                    Ok(None) => std::thread::sleep(Duration::from_millis(250)),
                    Err(_) => break None,
                }
            };
            let seconds = started.elapsed().as_secs();
            if let Some(f) = session.lock().unwrap().as_mut() {
                let _ = writeln!(f, "\n# exited with code {code:?} after {seconds} s");
            }
            use std::sync::atomic::Ordering::SeqCst;
            // Give the log readers a moment to finish the last lines.
            std::thread::sleep(Duration::from_millis(150));
            let findings = findings.lock().unwrap().clone();
            on_exit(ExitInfo { game_id, code, seconds, stopped: stopped.load(SeqCst), skip: skip.load(SeqCst), findings });
        });
    }

    Ok(Running {
        info: RunningInfo {
            game_id: req.game_id.to_string(),
            engine: req.engine_id.to_string(),
            engine_name: req.engine_name.to_string(),
            attempt: req.attempt,
            attempts: req.attempts,
            title: req.title.to_string(),
            pid,
            started_ms,
            command,
            notes,
            session_log: session_path.to_string_lossy().into_owned(),
        },
        child,
        started,
        stopped,
        skip,
        _guard: guard,
    })
}

/// Explain common exit codes in plain words.
pub fn explain_exit(code: Option<i32>) -> String {
    match code {
        Some(0) => "The game closed normally.".into(),
        None => "The game was stopped.".into(),
        Some(c) => {
            let u = c as u32;
            let what = match u {
                0xC000_0005 => "access violation (the game or engine crashed)",
                0xC000_001D => "illegal instruction — try CPU compatibility mode",
                0xC000_0409 => "stack buffer overrun / fast fail",
                0xC000_00FD => "stack overflow",
                0xC000_0017 | 0xC000_012D => "out of memory — raise or turn off the RAM limit",
                0xC000_0135 => "a DLL is missing next to the engine",
                0xC000_013A => "closed with Ctrl+C",
                1 => "engine reported an error (see Developer → Logs)",
                _ => "unexpected exit (see Developer → Logs)",
            };
            format!("Exited with code {c} (0x{u:08X}): {what}.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_parsing() {
        assert_eq!(parse_resolution("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_resolution(" 2560 X 1440 "), Some((2560, 1440)));
        assert_eq!(parse_resolution("100x100"), None);
        assert_eq!(parse_resolution("wide"), None);
    }

    #[test]
    fn args_match_official_launcher() {
        let mut e = EmuSettings::default();
        e.resolution = "1920x1080".into();
        e.gpu_index = 1;
        e.amd_cpu_compat = true;
        e.pro_mode = true;
        e.upscaler = "FSR".into();
        e.fsr_folder = "C:/fsr".into();
        let dev = DevSettings::default();
        let a = build_args(&e, &dev, "D:/G/eboot.bin", Path::new("/logs"));
        let s = a.join(" ");
        assert!(s.starts_with("--screen-width 1920 --screen-height 1080 --user-name Player"));
        assert!(s.contains("--gpu 1"));
        assert!(s.contains("--present-mode Mailbox"));
        assert!(s.contains("--shader-cache"));
        assert!(s.contains("--skip-notice-screen"));
        assert!(s.contains("--trinity --pssr-upscaler FSR --fsr-folder C:/fsr"));
        assert!(s.contains("--amd-cpu"));
        assert!(s.contains("--printf-direction Console"));
        assert!(!s.contains("--readback"));
        assert!(!s.contains("--no-trophy-notifications"));
        assert_eq!(a[a.len() - 2], "--game");
        assert_eq!(a.last().unwrap(), "D:/G/eboot.bin");
        // Every flag the engine knows takes a value or none; no empty values sneak in.
        assert!(a.iter().all(|x| !x.is_empty()));
    }

    #[test]
    fn auto_gpu_omits_flag() {
        let a = build_args(&EmuSettings::default(), &DevSettings::default(), "x", Path::new("/l"));
        assert!(!a.contains(&"--gpu".to_string()));
    }

    #[test]
    fn target_for_folder_and_archive() {
        assert!(game_target("D:/Games/A", false).ends_with("eboot.bin"));
        assert_eq!(game_target("D:/Games/A.zar", true), "D:/Games/A.zar");
    }

    #[test]
    fn exit_codes() {
        assert!(explain_exit(Some(0xC000_001Du32 as i32)).contains("CPU compatibility"));
        assert!(explain_exit(Some(0)).contains("normally"));
    }

    #[test]
    fn finds_engine_in_nested_folder() {
        let root = std::env::temp_dir().join(format!("strata_eng_{}", std::process::id()));
        let nested = root.join("engine/KytyPS5-2026-x64");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join(ENGINE_EXE), b"").unwrap();
        assert_eq!(find_engine("", &root), Some(nested.join(ENGINE_EXE)));
        assert_eq!(find_engine(&root.to_string_lossy(), Path::new("/nope")), Some(nested.join(ENGINE_EXE)));
        let _ = std::fs::remove_dir_all(root);
    }
}
