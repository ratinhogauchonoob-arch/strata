//! The engines Strata can drive. Each one is a separate open-source project (all GPL-2.0),
//! downloaded from its own GitHub releases and started through its command line.
//!
//! To add an engine: add an entry to `ENGINES`, a match arm in `plan()`, and (if it keeps
//! saves or controls somewhere) arms in `user_dirs()` and `controls::write_for()`.

use crate::games::Game;
use crate::settings::{DevSettings, EmuSettings};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Ps5,
    Ps4,
}

impl Platform {
    pub fn parse(s: &str) -> Platform {
        if s.eq_ignore_ascii_case("ps4") {
            Platform::Ps4
        } else {
            Platform::Ps5
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Ps5 => "ps5",
            Platform::Ps4 => "ps4",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Kyty,
    SharpEmu,
    AnyPs5,
    ShadPs4,
}

#[derive(Debug, Serialize)]
pub struct EngineDef {
    pub id: &'static str,
    pub name: &'static str,
    pub repo: &'static str,
    pub platforms: &'static [Platform],
    /// Can boot .zar archives directly.
    pub archives: bool,
    /// File names of the executable Strata starts (matched without case).
    pub exe_names: &'static [&'static str],
    pub summary: &'static str,
    #[serde(skip)]
    pub kind: Kind,
}

pub const ENGINES: &[EngineDef] = &[
    EngineDef {
        id: "kyty",
        name: "KytyPS5",
        repo: "KytyPS5/KytyPS5",
        platforms: &[Platform::Ps5],
        archives: true,
        exe_names: &["kyty_emulator.exe", "kyty_emulator"],
        summary: "PS5 translation layer. Goes in-game in many titles. Uses every setting in Strata.",
        kind: Kind::Kyty,
    },
    EngineDef {
        id: "sharpemu",
        name: "SharpEmu",
        repo: "sharpemu/sharpemu",
        platforms: &[Platform::Ps5, Platform::Ps4],
        archives: true,
        exe_names: &["sharpemu.exe", "sharpemu"],
        summary: "PS5 emulator written in C#, with an experimental PS4 mode.",
        kind: Kind::SharpEmu,
    },
    EngineDef {
        id: "anyps5",
        name: "AnyPS5",
        repo: "boykopovar/AnyPS5",
        platforms: &[Platform::Ps5],
        archives: false,
        exe_names: &["relinker.exe", "relinker"],
        summary: "Converts the game into a native program once, then runs it. Very early.",
        kind: Kind::AnyPs5,
    },
    EngineDef {
        id: "shadps4",
        name: "shadPS4",
        repo: "shadps4-emu/shadPS4",
        platforms: &[Platform::Ps4],
        archives: false,
        exe_names: &["shadps4.exe", "shadps4", "shadps4-sdl.appimage", "shadps4.appimage"],
        summary: "The most mature PS4 emulator. Hundreds of games are playable.",
        kind: Kind::ShadPs4,
    },
];

pub fn def(id: &str) -> Option<&'static EngineDef> {
    ENGINES.iter().find(|e| e.id == id)
}

pub fn default_order(p: Platform) -> Vec<String> {
    let ids: &[&str] = match p {
        Platform::Ps5 => &["kyty", "sharpemu", "anyps5"],
        Platform::Ps4 => &["shadps4", "sharpemu"],
    };
    ids.iter().map(|s| s.to_string()).collect()
}

/// Root folder Strata keeps for an engine: `bin/` holds the release, `user/` its saves and caches.
pub fn root_dir(data: &Path, id: &str) -> PathBuf {
    if id == "kyty" {
        data.join("engine") // layout from earlier versions, kept so existing installs still work
    } else {
        data.join("engines").join(id)
    }
}

pub fn bin_dir(data: &Path, id: &str) -> PathBuf {
    if id == "kyty" {
        root_dir(data, id)
    } else {
        root_dir(data, id).join("bin")
    }
}

fn matches_exe(def: &EngineDef, p: &Path) -> bool {
    p.is_file()
        && p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).is_some_and(|n| def.exe_names.contains(&n.as_str()))
}

fn search(def: &EngineDef, dir: &Path, depth: usize) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    // Newest release folders sort last by name (dates and versions), so prefer them.
    entries.reverse();
    if let Some(f) = entries.iter().find(|p| matches_exe(def, p)) {
        return Some(f.clone());
    }
    if depth == 0 {
        return None;
    }
    entries.iter().filter(|p| p.is_dir()).filter(|p| !p.ends_with("user")).find_map(|d| search(def, d, depth - 1))
}

/// Finds the engine: the path set by the user first, then Strata's own folders.
pub fn find(def: &EngineDef, configured: &str, data: &Path) -> Option<PathBuf> {
    let c = configured.trim().trim_matches('"');
    if !c.is_empty() {
        let p = PathBuf::from(c);
        if p.is_file() {
            return Some(p);
        }
        if p.is_dir() {
            if let Some(f) = search(def, &p, 3) {
                return Some(f);
            }
        }
    }
    if def.kind == Kind::Kyty {
        return crate::engine::find_engine("", data);
    }
    search(def, &bin_dir(data, def.id), 3)
}

pub fn supports(def: &EngineDef, game: &Game) -> bool {
    def.platforms.contains(&Platform::parse(&game.platform)) && (!game.archive || def.archives)
}

/// What to run for one attempt.
#[derive(Debug, Clone)]
pub struct Plan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    /// AnyPS5 only: convert first with these arguments (program = relinker).
    pub convert: Option<Vec<String>>,
}

pub fn eboot(game: &Game) -> String {
    crate::engine::game_target(&game.path, game.archive)
}

/// Where AnyPS5 writes the converted program of a game.
pub fn anyps5_output(data: &Path, game: &Game) -> PathBuf {
    let key = if game.title_id.is_empty() { safe_key(&game.id) } else { game.title_id.clone() };
    let name = if cfg!(windows) { "eboot.exe" } else { "eboot.elf" };
    root_dir(data, "anyps5").join("apps").join(key).join(name)
}

/// A short file-system-safe key for a path (games without a title id).
pub fn safe_key(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    format!("PATH-{h:016x}")
}

pub struct PlanInput<'a> {
    pub exe: &'a Path,
    pub game: &'a Game,
    pub emu: &'a EmuSettings,
    pub dev: &'a DevSettings,
    pub data: &'a Path,
    pub controls_file: Option<PathBuf>,
}

pub fn plan(def: &EngineDef, i: PlanInput) -> Plan {
    let exe_dir = i.exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let root = root_dir(i.data, def.id);
    let logs_dir = crate::logs::logs_dir(i.data);
    match def.kind {
        Kind::Kyty => Plan {
            program: i.exe.to_path_buf(),
            args: crate::engine::build_args(i.emu, i.dev, &eboot(i.game), &logs_dir),
            cwd: exe_dir,
            env: vec![],
            convert: None,
        },
        Kind::SharpEmu => {
            let mut env = vec![("SHARPEMU_USER_DIR".to_string(), root.join("user").to_string_lossy().into_owned())];
            if i.emu.perf_overlay {
                env.push(("SHARPEMU_LOG_VIDEOOUT_FPS".into(), "1".into()));
            }
            Plan { program: i.exe.to_path_buf(), args: vec![eboot(i.game)], cwd: exe_dir, env, convert: None }
        }
        Kind::ShadPs4 => {
            // shadPS4 keeps its data in "<working folder>/user", so run it from the engine root:
            // updating the release (bin/) then never touches saves or controls.
            let mut args = vec!["--fullscreen".to_string(), i.emu.fullscreen.to_string()];
            args.push("-g".into());
            args.push(eboot(i.game));
            Plan { program: i.exe.to_path_buf(), args, cwd: root, env: vec![], convert: None }
        }
        Kind::AnyPs5 => {
            let out = anyps5_output(i.data, i.game);
            let mut conv = Vec::new();
            if cfg!(windows) {
                conv.push("--windows".to_string());
            } else if cfg!(target_os = "macos") {
                conv.push("--macos".to_string());
            }
            let libs = exe_dir.join("libs");
            if libs.is_dir() && libs.to_string_lossy().is_ascii() {
                conv.push("--rpath".into());
                conv.push(libs.to_string_lossy().into_owned());
            }
            conv.push(eboot(i.game));
            conv.push(out.to_string_lossy().into_owned());
            let mut env = vec![("ANYPS5_AUTORUN_NO_PAUSE".to_string(), "1".to_string())];
            if let Some(c) = &i.controls_file {
                env.push(("ANYPS5_INPUT_CONFIG".into(), c.to_string_lossy().into_owned()));
            }
            Plan {
                program: out.clone(),
                args: vec![],
                cwd: out.parent().map(Path::to_path_buf).unwrap_or_default(),
                env,
                convert: Some(conv),
            }
        }
    }
}

/// Folders where an engine keeps saves and shader caches, if Strata knows them.
pub fn user_dirs(def: &EngineDef, data: &Path, exe: Option<&Path>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    match def.kind {
        Kind::ShadPs4 | Kind::SharpEmu => v.push(root_dir(data, def.id).join("user")),
        Kind::AnyPs5 => v.push(root_dir(data, def.id).join("apps")),
        Kind::Kyty => {
            if let Some(d) = exe.and_then(Path::parent) {
                v.push(d.to_path_buf());
            }
        }
    }
    v.into_iter().filter(|p| p.is_dir()).collect()
}

/// Engines that may run a game, best first. `forced` is a per-game choice ("" = automatic).
pub fn chain(game: &Game, order: &[String], forced: &str, last_good: &str, fallback: bool, installed: &dyn Fn(&str) -> bool) -> Vec<String> {
    if !forced.is_empty() {
        return def(forced).filter(|d| supports(d, game)).map(|d| vec![d.id.to_string()]).unwrap_or_default();
    }
    let mut ids: Vec<String> = Vec::new();
    let candidates = order.iter().cloned().chain(default_order(Platform::parse(&game.platform)));
    for id in candidates {
        if ids.contains(&id) {
            continue;
        }
        if let Some(d) = def(&id) {
            if supports(d, game) && installed(d.id) {
                ids.push(id);
            }
        }
    }
    if let Some(pos) = ids.iter().position(|x| x == last_good) {
        let good = ids.remove(pos);
        ids.insert(0, good);
    }
    if !fallback {
        ids.truncate(1);
    }
    ids
}

/// Did this attempt fail early enough that the next engine should get a go?
pub fn attempt_failed(code: Option<i32>, seconds: u64, fail_seconds: u32, stopped_by_user: bool, skip_requested: bool) -> bool {
    if skip_requested {
        return true;
    }
    if stopped_by_user {
        return false;
    }
    seconds < fail_seconds as u64 && code != Some(0) || seconds < 3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(platform: &str, archive: bool) -> Game {
        Game {
            id: "g".into(),
            path: "D:/G".into(),
            title: "G".into(),
            title_id: "PPSA00001".into(),
            version: String::new(),
            firmware: String::new(),
            archive,
            icon: None,
            background: None,
            logo: None,
            size_mb: 0,
            folder: String::new(),
            platform: platform.into(),
        }
    }

    #[test]
    fn chain_respects_platform_order_and_install() {
        let all = |_: &str| true;
        let ps5 = game("ps5", false);
        assert_eq!(chain(&ps5, &default_order(Platform::Ps5), "", "", true, &all), ["kyty", "sharpemu", "anyps5"]);
        let ps4 = game("ps4", false);
        assert_eq!(chain(&ps4, &default_order(Platform::Ps4), "", "", true, &all), ["shadps4", "sharpemu"]);
        // Only installed engines; last good engine goes first.
        let some = |id: &str| id != "kyty";
        assert_eq!(chain(&ps5, &default_order(Platform::Ps5), "", "anyps5", true, &some), ["anyps5", "sharpemu"]);
        // No fallback: one engine only.
        assert_eq!(chain(&ps5, &default_order(Platform::Ps5), "", "", false, &all), ["kyty"]);
        // Archives skip engines that cannot read them.
        assert_eq!(chain(&game("ps5", true), &default_order(Platform::Ps5), "", "", true, &all), ["kyty", "sharpemu"]);
        // Forced engine, and a forced engine that does not fit the game.
        assert_eq!(chain(&ps5, &[], "sharpemu", "", true, &all), ["sharpemu"]);
        assert!(chain(&ps5, &[], "shadps4", "", true, &all).is_empty());
        // Unknown ids in a hand-edited order are ignored; defaults fill the rest.
        assert_eq!(chain(&ps4, &["nope".into(), "sharpemu".into()], "", "", true, &all), ["sharpemu", "shadps4"]);
    }

    #[test]
    fn failure_rules() {
        assert!(attempt_failed(Some(1), 5, 20, false, false));
        assert!(attempt_failed(None, 5, 20, false, false), "crash without code");
        assert!(attempt_failed(Some(0), 1, 20, false, false), "closed instantly");
        assert!(!attempt_failed(Some(0), 10, 20, false, false), "closed normally");
        assert!(!attempt_failed(Some(1), 300, 20, false, false), "played a while");
        assert!(!attempt_failed(None, 5, 20, true, false), "user pressed Stop");
        assert!(attempt_failed(None, 500, 20, true, true), "user asked for the next engine");
    }

    #[test]
    fn plans() {
        let data = Path::new("/d");
        let e = EmuSettings { fullscreen: true, ..Default::default() };
        let dev = DevSettings::default();
        let g = game("ps4", false);
        let p = plan(def("shadps4").unwrap(), PlanInput { exe: Path::new("/d/engines/shadps4/bin/shadPS4"), game: &g, emu: &e, dev: &dev, data, controls_file: None });
        assert_eq!(p.args, ["--fullscreen", "true", "-g", &eboot(&g)]);
        assert_eq!(p.cwd, Path::new("/d/engines/shadps4"));

        let g5 = game("ps5", false);
        let p = plan(def("sharpemu").unwrap(), PlanInput { exe: Path::new("/x/SharpEmu"), game: &g5, emu: &e, dev: &dev, data, controls_file: None });
        assert_eq!(p.args, [eboot(&g5)]);
        assert!(p.env.iter().any(|(k, v)| k == "SHARPEMU_USER_DIR" && v.ends_with("user")));

        let p = plan(def("anyps5").unwrap(), PlanInput { exe: Path::new("/x/relinker"), game: &g5, emu: &e, dev: &dev, data, controls_file: Some("/c.ini".into()) });
        let conv = p.convert.unwrap();
        assert_eq!(conv[conv.len() - 2], eboot(&g5));
        assert_eq!(PathBuf::from(&conv[conv.len() - 1]), p.program);
        assert!(p.program.to_string_lossy().contains("PPSA00001"));
        assert!(p.env.iter().any(|(k, _)| k == "ANYPS5_INPUT_CONFIG"));

        let p = plan(def("kyty").unwrap(), PlanInput { exe: Path::new("/x/kyty_emulator"), game: &g5, emu: &e, dev: &dev, data, controls_file: None });
        assert_eq!(p.args.last().unwrap(), &eboot(&g5));
        assert!(p.args.contains(&"--fullscreen".to_string()));
    }

    #[test]
    fn finds_engines_without_case_and_prefers_newest() {
        let root = std::env::temp_dir().join(format!("strata_engs_{}", std::process::id()));
        let d = def("shadps4").unwrap();
        let old = bin_dir(&root, "shadps4").join("shadps4-v0.19");
        let new = bin_dir(&root, "shadps4").join("shadps4-v0.20");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("shadPS4.exe"), b"").unwrap();
        std::fs::write(new.join("ShadPS4.EXE"), b"").unwrap();
        assert_eq!(find(d, "", &root), Some(new.join("ShadPS4.EXE")));
        assert!(find(def("sharpemu").unwrap(), "", &root).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn keys_are_stable() {
        assert_eq!(safe_key("D:/a"), safe_key("D:/a"));
        assert_ne!(safe_key("D:/a"), safe_key("D:/b"));
    }
}
