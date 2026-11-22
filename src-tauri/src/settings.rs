//! Persistent settings. Everything lives in one JSON file inside the data folder
//! (portable: next to the executable when that folder is writable).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const SETTINGS_VERSION: u32 = 2;

/// Engine options. Field names map 1:1 to engine command-line switches (see engine.rs).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EmuSettings {
    // Display
    pub resolution: String,
    pub present_mode: String, // Fifo | Mailbox | Immediate
    pub gpu_index: i32,       // -1 = automatic
    pub fullscreen: bool,
    pub stretch: bool,
    pub hdr: bool,
    pub hide_cursor: bool,
    pub vblank_frequency: u32,
    // Rendering
    pub shader_cache: bool,
    pub relaxed_shader_float: bool,
    pub readback_linear: bool,
    pub readback_tiled: bool,
    pub readback_storage: bool,
    pub sync_raw_image_buffers: bool,
    pub flush_draws: u32,
    pub tessellation: bool,
    pub occlusion_queries: bool,
    pub native_rt: bool,
    pub pro_mode: bool,
    pub upscaler: String, // Linear | DLSS | FSR
    pub dlss_folder: String,
    pub fsr_folder: String,
    // CPU / timing
    pub amd_cpu_compat: bool,
    pub guest_tsc: bool,
    pub red_zone: bool,
    // System
    pub user_name: String,
    pub user_id: u32,
    pub console_language: u32,
    pub network: bool,
    pub emulate_online: bool,
    pub skip_notice_screen: bool,
    pub trophies: bool,
    pub trophy_duration: u32,
    pub perf_overlay: bool,
    // Controller
    pub keyboard_player: u32,
    pub vibration: u32,
    pub speaker_volume: u32,
    pub controller_color: String,
    // Debug (engine side)
    pub vulkan_validation: bool,
    pub shader_validation: bool,
    pub shader_log: String, // Silent | Console | File
    pub command_buffer_dump: bool,
    pub profiler: bool,
    pub renderdoc: bool,
}

impl Default for EmuSettings {
    fn default() -> Self {
        Self {
            resolution: "1280x720".into(),
            present_mode: "Mailbox".into(),
            gpu_index: -1,
            fullscreen: false,
            stretch: false,
            hdr: false,
            hide_cursor: true,
            vblank_frequency: 0,
            shader_cache: true,
            relaxed_shader_float: false,
            readback_linear: false,
            readback_tiled: false,
            readback_storage: false,
            sync_raw_image_buffers: false,
            flush_draws: 0,
            tessellation: false,
            occlusion_queries: false,
            native_rt: false,
            pro_mode: false,
            upscaler: "Linear".into(),
            dlss_folder: String::new(),
            fsr_folder: String::new(),
            amd_cpu_compat: false,
            guest_tsc: false,
            red_zone: false,
            user_name: "Player".into(),
            user_id: 1,
            console_language: 1,
            network: false,
            emulate_online: false,
            skip_notice_screen: true,
            trophies: true,
            trophy_duration: 6,
            perf_overlay: false,
            keyboard_player: 0,
            vibration: 100,
            speaker_volume: 50,
            controller_color: String::new(),
            vulkan_validation: false,
            shader_validation: false,
            shader_log: "Silent".into(),
            command_buffer_dump: false,
            profiler: false,
            renderdoc: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PerfSettings {
    pub ram_limit_enabled: bool,
    pub ram_limit_gb: u32,
    /// "soft": keep physical RAM under the limit, the rest goes to the page file.
    /// "strict": the game is closed if it commits more than the limit.
    pub ram_mode: String,
    /// "normal" | "above_normal" | "high"
    pub priority: String,
}

impl Default for PerfSettings {
    fn default() -> Self {
        Self { ram_limit_enabled: false, ram_limit_gb: 8, ram_mode: "soft".into(), priority: "normal".into() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UiSettings {
    pub accent1: String,
    pub accent2: String,
    /// "duo" (accent 1 + 2), "spectrum" (all six), "off"
    pub boot_style: String,
    pub reduce_motion: bool,
    pub clock_24h: bool,
    pub ui_sounds: bool,
    /// Big text, full screen, controller-first ("TV mode").
    pub tv_mode: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            accent1: "ember".into(),
            accent2: "cobalt".into(),
            boot_style: "duo".into(),
            reduce_motion: false,
            clock_24h: true,
            ui_sounds: true,
            tv_mode: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DevSettings {
    pub enabled: bool,
    /// "info" | "debug" | "trace"
    pub log_level: String,
    /// Ask the engine to print its own log to stdout so the dev console can show it.
    pub capture_engine_log: bool,
    pub keep_sessions: u32,
    pub show_command_line: bool,
}

impl Default for DevSettings {
    fn default() -> Self {
        Self { enabled: false, log_level: "info".into(), capture_engine_log: true, keep_sessions: 20, show_command_line: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EngineSettings {
    /// Engine ids to try for PS5 games, best first.
    pub order_ps5: Vec<String>,
    /// Engine ids to try for PS4 games, best first.
    pub order_ps4: Vec<String>,
    /// When an engine fails early, try the next one.
    pub auto_fallback: bool,
    /// An engine that closes before this many seconds counts as failed.
    pub fail_seconds: u32,
    /// Folder or executable chosen by the user, per engine id.
    pub paths: BTreeMap<String, String>,
    /// The user said they do not play PS4 games (setup skipped the PS4 engine).
    pub skip_ps4: bool,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            order_ps5: crate::engines::default_order(crate::engines::Platform::Ps5),
            order_ps4: crate::engines::default_order(crate::engines::Platform::Ps4),
            auto_fallback: true,
            fail_seconds: 20,
            paths: BTreeMap::new(),
            skip_ps4: false,
        }
    }
}

/// One keyboard/mouse binding table, shared by every engine that lets Strata write its controls.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ControlSettings {
    /// Strata writes the engines' control files (otherwise their own defaults stay).
    pub enabled: bool,
    /// action -> key name (see controls::KEYS)
    pub keys: BTreeMap<String, String>,
    pub swap_cross_circle: bool,
    pub deadzone: u32,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self { enabled: false, keys: crate::controls::default_keys(), swap_cross_circle: false, deadzone: 8 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CommunitySettings {
    /// Send anonymous test results to the website (off until the user says yes).
    pub share: bool,
    /// Use everyone's combined results to pick engines.
    pub use_results: bool,
    /// The one-time question was answered.
    pub asked: bool,
    /// Show "Playing <game> on Strata" in Discord.
    pub discord: bool,
}

impl Default for CommunitySettings {
    fn default() -> Self {
        Self { share: false, use_results: true, asked: false, discord: false }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GameEntry {
    pub favorite: bool,
    pub last_played: Option<i64>,
    pub play_seconds: u64,
    pub launches: u32,
    /// "unknown" | "boots" | "menus" | "playable" | "broken"
    pub status: String,
    pub note: String,
    /// When set, these replace the global engine settings for this game.
    pub emu: Option<EmuSettings>,
    pub perf: Option<PerfSettings>,
    /// Engine chosen for this game ("" = automatic).
    pub engine: String,
    /// Engine that last ran this game past the failure window.
    pub good_engine: String,
    pub controls: Option<ControlSettings>,
    /// shadPS4 patch file (XML) for this game ("" = none).
    pub patch_file: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub first_run: bool,
    /// Path to kyty_emulator(.exe). Empty = look in <app>/engine.
    pub engine_path: String,
    pub library_folders: Vec<String>,
    pub ui: UiSettings,
    pub emu: EmuSettings,
    pub perf: PerfSettings,
    pub dev: DevSettings,
    /// Tier name of the last automatic profile, empty if never applied.
    pub auto_profile: String,
    pub engines: EngineSettings,
    pub controls: ControlSettings,
    /// Where "Install .pkg" puts games ("" = automatic: a library folder on another drive first).
    pub pkg_install_folder: String,
    pub community: CommunitySettings,
    pub games: BTreeMap<String, GameEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            first_run: true,
            engine_path: String::new(),
            library_folders: Vec::new(),
            ui: UiSettings::default(),
            emu: EmuSettings::default(),
            perf: PerfSettings::default(),
            dev: DevSettings::default(),
            auto_profile: String::new(),
            engines: EngineSettings::default(),
            controls: ControlSettings::default(),
            pkg_install_folder: String::new(),
            community: CommunitySettings::default(),
            games: BTreeMap::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(mut s) => {
                    s.migrate();
                    s.sanitize();
                    s
                }
                Err(e) => {
                    // Keep the broken file for inspection instead of silently losing it.
                    let _ = fs::copy(path, path.with_extension("broken.json"));
                    crate::logs::warn("settings", &format!("settings.json unreadable ({e}); using defaults"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap_or_default())?;
        fs::rename(&tmp, path)
    }

    /// One-time changes for files written by older versions (runs before sanitize).
    pub fn migrate(&mut self) {
        if self.version < 2 {
            // 0.9 always used 60 Hz; 0.10 matches the monitor instead.
            if self.emu.vblank_frequency == 60 {
                self.emu.vblank_frequency = 0;
            }
            for g in self.games.values_mut() {
                if let Some(e) = g.emu.as_mut().filter(|e| e.vblank_frequency == 60) {
                    e.vblank_frequency = 0;
                }
            }
        }
    }

    /// Clamp values a hand-edited file could break.
    pub fn sanitize(&mut self) {
        self.version = SETTINGS_VERSION;
        let fix = |e: &mut EmuSettings| {
            if crate::engine::parse_resolution(&e.resolution).is_none() {
                e.resolution = "1280x720".into();
            }
            if !["Fifo", "Mailbox", "Immediate"].contains(&e.present_mode.as_str()) {
                e.present_mode = "Mailbox".into();
            }
            if !["Linear", "DLSS", "FSR"].contains(&e.upscaler.as_str()) {
                e.upscaler = "Linear".into();
            }
            if !["Silent", "Console", "File"].contains(&e.shader_log.as_str()) {
                e.shader_log = "Silent".into();
            }
            // 0 = Auto (the refresh rate of the monitor Strata is on).
            if e.vblank_frequency != 0 {
                e.vblank_frequency = e.vblank_frequency.clamp(24, 500);
            }
            e.vibration = e.vibration.min(100);
            e.speaker_volume = e.speaker_volume.min(100);
            e.console_language = e.console_language.clamp(0, 29);
            e.keyboard_player = e.keyboard_player.min(4);
            e.trophy_duration = e.trophy_duration.clamp(1, 60);
        };
        fix(&mut self.emu);
        // Older versions kept one engine path; it belongs to KytyPS5.
        if !self.engine_path.trim().is_empty() && !self.engines.paths.contains_key("kyty") {
            self.engines.paths.insert("kyty".into(), self.engine_path.clone());
        }
        let known = |v: &mut Vec<String>| {
            v.retain(|id| crate::engines::def(id).is_some());
            let mut seen = Vec::new();
            v.retain(|id| if seen.contains(id) { false } else { seen.push(id.clone()); true });
        };
        known(&mut self.engines.order_ps5);
        known(&mut self.engines.order_ps4);
        self.engines.fail_seconds = self.engines.fail_seconds.clamp(5, 300);
        self.engines.paths.retain(|k, v| crate::engines::def(k).is_some() && !v.trim().is_empty());
        let fixc = |c: &mut ControlSettings| {
            c.deadzone = c.deadzone.min(90);
            c.keys.retain(|a, k| crate::controls::is_action(a) && crate::controls::is_key(k));
        };
        fixc(&mut self.controls);
        self.perf.ram_limit_gb = self.perf.ram_limit_gb.clamp(2, 256);
        for g in self.games.values_mut() {
            if let Some(e) = g.emu.as_mut() {
                fix(e);
            }
            if let Some(p) = g.perf.as_mut() {
                p.ram_limit_gb = p.ram_limit_gb.clamp(2, 256);
            }
            if let Some(c) = g.controls.as_mut() {
                fixc(c);
            }
            if !g.engine.is_empty() && crate::engines::def(&g.engine).is_none() {
                g.engine.clear();
            }
        }
    }

    /// Engine settings that apply to one game (its override or the global ones).
    pub fn emu_for(&self, game_id: &str) -> EmuSettings {
        self.games.get(game_id).and_then(|g| g.emu.clone()).unwrap_or_else(|| self.emu.clone())
    }

    pub fn controls_for(&self, game_id: &str) -> ControlSettings {
        self.games.get(game_id).and_then(|g| g.controls.clone()).unwrap_or_else(|| self.controls.clone())
    }

    pub fn engine_path(&self, id: &str) -> String {
        self.engines.paths.get(id).cloned().unwrap_or_default()
    }

    pub fn perf_for(&self, game_id: &str) -> PerfSettings {
        self.games.get(game_id).and_then(|g| g.perf.clone()).unwrap_or_else(|| self.perf.clone())
    }
}

/// Folder for settings, logs, default game folder and the engine.
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("STRATA_DATA") {
        return PathBuf::from(dir);
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        let portable = exe_dir.join("data");
        if fs::create_dir_all(&portable).is_ok() && is_writable(&portable) {
            return portable;
        }
    }
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("XDG_DATA_HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("Strata")
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".write_test");
    let ok = fs::write(&probe, b"ok").is_ok();
    let _ = fs::remove_file(probe);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_partial_file() {
        let dir = std::env::temp_dir().join(format!("strata_set_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings.json");
        let mut s = Settings::default();
        s.emu.resolution = "1920x1080".into();
        s.library_folders.push("D:/Games".into());
        s.save(&p).unwrap();
        assert_eq!(Settings::load(&p), s);

        // Older/partial files keep defaults for missing fields.
        fs::write(&p, r#"{"emu":{"resolution":"2560x1440"},"perf":{"ram_limit_gb":1}}"#).unwrap();
        let l = Settings::load(&p);
        assert_eq!(l.emu.resolution, "2560x1440");
        assert_eq!(l.emu.present_mode, "Mailbox");
        assert_eq!(l.perf.ram_limit_gb, 2, "clamped");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sanitize_fixes_garbage() {
        let mut s = Settings::default();
        s.emu.resolution = "big".into();
        s.emu.present_mode = "Turbo".into();
        s.emu.vibration = 400;
        s.sanitize();
        assert_eq!(s.emu.resolution, "1280x720");
        assert_eq!(s.emu.present_mode, "Mailbox");
        assert_eq!(s.emu.vibration, 100);
        let mut s = Settings::default();
        s.emu.vblank_frequency = 0;
        s.engine_path = "C:/kyty".into();
        s.engines.order_ps5 = vec!["nope".into(), "kyty".into(), "kyty".into()];
        s.engines.fail_seconds = 1;
        s.sanitize();
        assert_eq!(s.emu.vblank_frequency, 0, "0 means Auto");
        assert_eq!(s.engines.paths["kyty"], "C:/kyty", "old engine path migrates");
        assert_eq!(s.engines.order_ps5, ["kyty"]);
        assert_eq!(s.engines.fail_seconds, 5);
        s.emu.vblank_frequency = 999;
        s.sanitize();
        assert_eq!(s.emu.vblank_frequency, 500);
    }

    #[test]
    fn per_game_override() {
        let mut s = Settings::default();
        let mut e = EmuSettings::default();
        e.resolution = "1920x1080".into();
        s.games.insert("g1".into(), GameEntry { emu: Some(e), ..Default::default() });
        assert_eq!(s.emu_for("g1").resolution, "1920x1080");
        assert_eq!(s.emu_for("g2").resolution, "1280x720");
    }
}
