//! Compatibility knowledge: what a game's eboot.bin needs, what each engine was missing when it
//! failed, and which engine ran which game. Strata uses it to start with the engine most likely
//! to work, and to open a game straight in the engine that worked last time.
//!
//! Files (plain JSON, readable and shareable):
//!   data/compat/<TITLE_ID>.json  — one game: analysis + every attempt
//!   data/compat/engines.json     — functions/modules each engine release was seen missing

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const ANALYSIS_VERSION: u32 = 1;

// ───────────────────────── binary analysis ─────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Analysis {
    pub version: u32,
    /// "elf" | "self-ps4" | "self-ps5" | "unknown"
    pub format: String,
    /// The program could be read (a decrypted dump). False = encrypted or not an executable.
    pub readable: bool,
    /// Imported system functions (NIDs), e.g. "qCSfqDILlns".
    pub imports: BTreeSet<String>,
    /// System libraries named in the executable, e.g. "libSceNpToolkit2".
    pub libraries: BTreeSet<String>,
    /// Modules shipped with the game in sce_module/.
    pub bundled: BTreeSet<String>,
    /// "Unreal Engine", "Unity", ... or "".
    pub game_engine: String,
    pub eboot_size: u64,
    pub eboot_mtime: u64,
}

fn mtime(p: &Path) -> u64 {
    p.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

fn is_nid_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'+' || c == b'-'
}

/// NIDs appear in the symbol strings of PS4/PS5 executables as "XXXXXXXXXXX#L#M" (11 base64 chars).
pub fn find_nids(b: &[u8]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i + 15 <= b.len() {
        if b[i] == b'#' && i >= 11 && (i < 12 || !is_nid_char(b[i - 12])) && b[i - 11..i].iter().all(|&c| is_nid_char(c)) {
            // "#lib#mod" must follow: short base64 ids
            let rest = &b[i + 1..b.len().min(i + 10)];
            let mut parts = rest.split(|&c| c == b'#');
            let lib = parts.next().unwrap_or_default();
            let ok_lib = !lib.is_empty() && lib.len() <= 3 && lib.iter().all(|&c| is_nid_char(c));
            let ok_mod = parts.next().is_some_and(|m| {
                let m: Vec<u8> = m.iter().copied().take_while(|&c| is_nid_char(c)).collect();
                !m.is_empty() && m.len() <= 3
            });
            if ok_lib && ok_mod {
                out.insert(String::from_utf8_lossy(&b[i - 11..i]).into_owned());
            }
        }
        i += 1;
    }
    out
}

/// "libSceXxx" / "libkernel" / "libc" names inside the executable.
pub fn find_libraries(b: &[u8]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let pats: [&[u8]; 3] = [b"libSce", b"libkernel", b"libc\0"];
    for pat in pats {
        let mut start = 0;
        while let Some(pos) = b[start..].windows(pat.len()).position(|w| w == pat) {
            let at = start + pos;
            let end = b[at..].iter().position(|&c| !(c.is_ascii_alphanumeric() || c == b'_')).map(|e| at + e).unwrap_or(b.len());
            let name = String::from_utf8_lossy(&b[at..end]).into_owned();
            if name.len() >= 4 && name.len() < 48 {
                out.insert(name);
            }
            start = at + pat.len();
        }
    }
    out
}

fn contains(b: &[u8], s: &str) -> bool {
    b.windows(s.len()).any(|w| w == s.as_bytes())
}

/// Which game engine made the game: from strings in the executable, then from the files around it.
pub fn detect_engine(b: &[u8], dir: &Path) -> String {
    let checks: [(&str, &[&str]); 7] = [
        ("Unreal Engine", &["UnrealEngine", "Unreal Engine", "FEngineLoop", "UE4Game", "UE5Game"]),
        ("Unity", &["UnityPlayer", "il2cpp", "Unity Technologies"]),
        ("GameMaker", &["YoYo Games", "YYGames"]),
        ("Godot", &["Godot Engine"]),
        ("CRYENGINE", &["CryEngine", "CRYENGINE"]),
        ("RE Engine", &["RE ENGINE", "via.Application"]),
        ("Decima", &["Decima"]),
    ];
    for (name, pats) in checks {
        if pats.iter().any(|p| contains(b, p)) {
            return name.into();
        }
    }
    let has = |rel: &str| dir.join(rel).exists();
    if has("Media/Data/Managed") || has("Media/Data/globalgamemanagers") {
        return "Unity".into();
    }
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && (p.join("Content/Paks").is_dir() || p.join("Binaries").is_dir()) {
                return "Unreal Engine".into();
            }
        }
    }
    String::new()
}

pub fn analyze(game_dir: &Path) -> Analysis {
    let eboot = game_dir.join("eboot.bin");
    let mut a = Analysis { version: ANALYSIS_VERSION, eboot_size: eboot.metadata().map(|m| m.len()).unwrap_or(0), eboot_mtime: mtime(&eboot), ..Default::default() };
    let Ok(b) = fs::read(&eboot) else {
        a.format = "unknown".into();
        return a;
    };
    a.format = match b.get(0..4) {
        Some([0x7F, b'E', b'L', b'F']) => "elf",
        Some([0x4F, 0x15, 0x3D, 0x1D]) => "self-ps4",
        Some([0x54, 0x14, 0xF5, 0xEE]) => "self-ps5",
        _ => "unknown",
    }
    .into();
    a.imports = find_nids(&b);
    a.libraries = find_libraries(&b);
    a.readable = !a.imports.is_empty();
    a.game_engine = detect_engine(&b, game_dir);
    for sub in ["sce_module", "sce_modules"] {
        if let Ok(rd) = fs::read_dir(game_dir.join(sub)) {
            a.bundled.extend(rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()));
        }
    }
    a
}

/// The analysis is still valid for this eboot.bin.
pub fn is_fresh(a: &Analysis, game_dir: &Path) -> bool {
    let eboot = game_dir.join("eboot.bin");
    a.version == ANALYSIS_VERSION && a.eboot_size == eboot.metadata().map(|m| m.len()).unwrap_or(0) && a.eboot_mtime == mtime(&eboot)
}

// ───────────────────────── what engines print when they fail ─────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Findings {
    pub missing_functions: BTreeSet<String>,
    pub missing_modules: BTreeSet<String>,
    /// Last fatal-looking line from the engine.
    pub last_error: String,
}

fn after<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.find(key).map(|i| line[i + key.len()..].trim_start())
}

fn first_token(s: &str) -> String {
    s.split(|c: char| c.is_whitespace() || c == ')' || c == ',' || c == '\'' || c == '"').next().unwrap_or("").to_string()
}

/// Reads one engine log line and remembers anything that explains a failure.
pub fn scan_line(line: &str, f: &mut Findings) {
    // SharpEmu: "Missing HLE export for NID: qCSfqDILlns"
    if let Some(r) = after(line, "Missing HLE export for NID:") {
        let t = first_token(r);
        if t.len() == 11 {
            f.missing_functions.insert(t);
        }
    }
    // shadPS4: "Can't Load libSceLibcInternal.sprx switching to HLE"
    if let Some(r) = after(line, "Can't Load ") {
        let t = first_token(r);
        if t.ends_with(".sprx") || t.ends_with(".prx") {
            f.missing_modules.insert(t);
        }
    }
    // Generic "unimplemented ... nid: XXXXXXXXXXX" / "NID XXXXXXXXXXX not implemented"
    let low = line.to_ascii_lowercase();
    if low.contains("unimplemented") || low.contains("not implemented") || low.contains("unresolved") {
        for key in ["nid: ", "nid=", "NID: ", "nid "] {
            if let Some(r) = after(line, key) {
                let t = first_token(r);
                if t.len() == 11 && t.bytes().all(is_nid_char) {
                    f.missing_functions.insert(t);
                }
            }
        }
    }
    if low.contains("critical") || low.contains("fatal") || low.contains("unhandled exception") || low.contains("assertion failed") {
        f.last_error = line.chars().take(300).collect();
    }
}

// ───────────────────────── per-game record ─────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Attempt {
    pub engine: String,
    /// Engine release folder (changes when the engine updates).
    pub release: String,
    pub when: u64,
    pub seconds: u64,
    pub code: Option<i32>,
    pub worked: bool,
    pub findings: Findings,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GameRecord {
    pub title: String,
    pub title_id: String,
    pub analysis: Analysis,
    /// Newest last; trimmed to the last 30.
    pub attempts: Vec<Attempt>,
}

pub fn dir(data: &Path) -> PathBuf {
    data.join("compat")
}

pub fn record_path(data: &Path, key: &str) -> PathBuf {
    let safe: String = key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    dir(data).join(format!("{safe}.json"))
}

pub fn load(data: &Path, key: &str) -> GameRecord {
    fs::read_to_string(record_path(data, key)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(data: &Path, key: &str, r: &GameRecord) -> Result<(), String> {
    fs::create_dir_all(dir(data)).map_err(|e| e.to_string())?;
    let p = record_path(data, key);
    let tmp = p.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(r).unwrap_or_default()).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

// ───────────────────────── engine knowledge (shared by every game) ─────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EngineKnowledge {
    /// engine id → release → functions it was seen missing
    pub missing_functions: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    pub missing_modules: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
}

pub fn knowledge_path(data: &Path) -> PathBuf {
    dir(data).join("engines.json")
}

pub fn load_knowledge(data: &Path) -> EngineKnowledge {
    fs::read_to_string(knowledge_path(data)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn learn(k: &mut EngineKnowledge, engine: &str, release: &str, f: &Findings) {
    if !f.missing_functions.is_empty() {
        k.missing_functions.entry(engine.into()).or_default().entry(release.into()).or_default().extend(f.missing_functions.iter().cloned());
    }
    if !f.missing_modules.is_empty() {
        k.missing_modules.entry(engine.into()).or_default().entry(release.into()).or_default().extend(f.missing_modules.iter().cloned());
    }
}

pub fn save_knowledge(data: &Path, k: &EngineKnowledge) -> Result<(), String> {
    fs::create_dir_all(dir(data)).map_err(|e| e.to_string())?;
    fs::write(knowledge_path(data), serde_json::to_vec_pretty(k).unwrap_or_default()).map_err(|e| e.to_string())
}

// ───────────────────────── scoring ─────────────────────────

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Score {
    pub engine: String,
    /// 0..=100
    pub score: u32,
    /// "worked" | "failed" | "high" | "medium" | "low" | "unknown"
    pub label: String,
    pub reason: String,
    /// Functions this game uses that this engine release was seen missing.
    pub known_missing: Vec<String>,
}

/// How likely an engine is to run a game. `release` = the installed engine release.
/// `others` = how many other players this engine ran the game for (community results).
pub fn score(engine: &str, release: &str, rec: &GameRecord, k: &EngineKnowledge, others: u32) -> Score {
    let mk = |score: u32, label: &str, reason: String, known: Vec<String>| Score { engine: engine.into(), score, label: label.into(), reason, known_missing: known };
    // 1. What happened with this very engine release on this game wins.
    if let Some(a) = rec.attempts.iter().rev().find(|a| a.engine == engine && a.release == release) {
        return if a.worked {
            mk(100, "worked", format!("Ran this game for {} s last time.", a.seconds), vec![])
        } else {
            let why = a.findings.missing_functions.iter().next().map(|n| format!(" (missing function {n})")).or_else(|| a.findings.missing_modules.iter().next().map(|m| format!(" (missing module {m})"))).unwrap_or_default();
            mk(5, "failed", format!("Closed after {} s last time{why}.", a.seconds), a.findings.missing_functions.iter().cloned().collect())
        };
    }
    // 2. Known gaps of this engine release that this game needs.
    let gaps: Vec<String> = k
        .missing_functions
        .get(engine)
        .and_then(|m| m.get(release))
        .map(|set| set.iter().filter(|n| rec.analysis.imports.contains(*n)).cloned().collect())
        .unwrap_or_default();
    if !gaps.is_empty() {
        return mk(20u32.saturating_sub(gaps.len().min(15) as u32), "low", format!("Needs {} function(s) this engine is known to miss.", gaps.len()), gaps);
    }
    // 3. It ran for other players (community results), or an older release ran it here.
    if others > 0 {
        return mk((70 + others.min(25)).min(95), "high", format!("Worked for {others} other player(s)."), vec![]);
    }
    if rec.attempts.iter().any(|a| a.engine == engine && a.worked) {
        return mk(80, "high", "An earlier version of this engine ran it.".into(), vec![]);
    }
    if rec.attempts.iter().any(|a| a.engine == engine && !a.worked) {
        return mk(35, "medium", "An earlier version failed; this one is newer.".into(), vec![]);
    }
    if !rec.analysis.readable {
        return mk(50, "unknown", "The game's program could not be read, so no prediction.".into(), vec![]);
    }
    mk(60, "medium", format!("No known gaps for the {} functions this game uses.", rec.analysis.imports.len()), vec![])
}

/// Sorts a chain by score, keeping the user's order for ties.
pub fn order(chain: &mut [String], scores: &[Score]) {
    let s = |id: &String| scores.iter().find(|x| &x.engine == id).map(|x| x.score).unwrap_or(50);
    chain.sort_by(|a, b| s(b).cmp(&s(a)));
}

pub fn add_attempt(rec: &mut GameRecord, a: Attempt) {
    rec.attempts.push(a);
    let n = rec.attempts.len();
    if n > 30 {
        rec.attempts.drain(0..n - 30);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_elf() -> Vec<u8> {
        let mut b = vec![0x7F, b'E', b'L', b'F'];
        b.extend_from_slice(&[0u8; 60]);
        b.extend_from_slice(b"\0qCSfqDILlns#A#B\0L1SBTkC+Cvw#C#D\0ab#c\0libSceNpToolkit2\0libkernel\0libc\0junk-not-a-nid#q\0");
        b.extend_from_slice(b"FEngineLoop\0");
        b
    }

    #[test]
    fn reads_imports_libraries_and_engine() {
        let b = fake_elf();
        let n = find_nids(&b);
        assert!(n.contains("qCSfqDILlns") && n.contains("L1SBTkC+Cvw"), "{n:?}");
        assert_eq!(n.len(), 2, "no false positives: {n:?}");
        let l = find_libraries(&b);
        assert!(l.contains("libSceNpToolkit2") && l.contains("libkernel") && l.contains("libc"), "{l:?}");

        let dir = std::env::temp_dir().join(format!("strata_compat_{}", std::process::id()));
        fs::create_dir_all(dir.join("sce_module")).unwrap();
        fs::write(dir.join("eboot.bin"), &b).unwrap();
        fs::write(dir.join("sce_module/libc.prx"), b"x").unwrap();
        let a = analyze(&dir);
        assert_eq!(a.format, "elf");
        assert!(a.readable);
        assert_eq!(a.game_engine, "Unreal Engine");
        assert!(a.bundled.contains("libc.prx"));
        assert!(is_fresh(&a, &dir));
        fs::write(dir.join("eboot.bin"), b"\x7fELF changed").unwrap();
        assert!(!is_fresh(&a, &dir), "an update invalidates the analysis");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn engine_logs_explain_failures() {
        let mut f = Findings::default();
        scan_line("[LOADER][ERROR] Missing HLE export for NID: qCSfqDILlns", &mut f);
        scan_line("[Core.Linker] <Info> linker.cpp:89 Execute: Can't Load libSceLibcInternal.sprx switching to HLE", &mut f);
        scan_line("[LOADER][WARN] Import#44 unresolved occurrence=1: nid=AbCdEfGhIjK ret=0x1", &mut f);
        scan_line("[Debug] <Critical> SignalHandler: Unhandled Exception code 0xc0000005", &mut f);
        scan_line("frame 60 · 16.6 ms", &mut f);
        assert!(f.missing_functions.contains("qCSfqDILlns") && f.missing_functions.contains("AbCdEfGhIjK"));
        assert!(f.missing_modules.contains("libSceLibcInternal.sprx"));
        assert!(f.last_error.contains("Unhandled Exception"));
    }

    #[test]
    fn scoring_and_order() {
        let mut rec = GameRecord::default();
        rec.analysis.readable = true;
        rec.analysis.imports = ["qCSfqDILlns".to_string(), "AAAAAAAAAAA".to_string()].into();
        let mut k = EngineKnowledge::default();
        // Another game taught us SharpEmu r1 misses qCSfqDILlns.
        learn(&mut k, "sharpemu", "r1", &Findings { missing_functions: ["qCSfqDILlns".to_string()].into(), ..Default::default() });
        assert_eq!(score("sharpemu", "r1", &rec, &k, 0).label, "low");
        assert_eq!(score("sharpemu", "r1", &rec, &k, 0).known_missing, ["qCSfqDILlns"]);
        assert_eq!(score("sharpemu", "r2", &rec, &k, 0).label, "medium", "a newer release may have it");
        assert_eq!(score("kyty", "k1", &rec, &k, 0).label, "medium");

        add_attempt(&mut rec, Attempt { engine: "kyty".into(), release: "k1".into(), seconds: 600, worked: true, ..Default::default() });
        assert_eq!(score("kyty", "k1", &rec, &k, 0).label, "worked");
        assert_eq!(score("kyty", "k2", &rec, &k, 0).label, "high", "engine updated: still likely");

        let mut chain = vec!["sharpemu".to_string(), "anyps5".to_string(), "kyty".to_string()];
        let scores: Vec<Score> = chain.iter().map(|e| score(e, if e == "kyty" { "k1" } else { "r1" }, &rec, &k, 0)).collect();
        order(&mut chain, &scores);
        assert_eq!(chain, ["kyty", "anyps5", "sharpemu"]);
        // Community: worked for 3 other players and nothing known locally.
        let fresh = GameRecord { analysis: rec.analysis.clone(), ..Default::default() };
        assert_eq!(score("anyps5", "a1", &fresh, &k, 3).label, "high");
        assert!(score("anyps5", "a1", &fresh, &k, 3).score > score("anyps5", "a1", &fresh, &k, 0).score);

        for i in 0..40 {
            add_attempt(&mut rec, Attempt { engine: format!("e{i}"), ..Default::default() });
        }
        assert_eq!(rec.attempts.len(), 30);
    }

    #[test]
    fn records_roundtrip() {
        let data = std::env::temp_dir().join(format!("strata_rec_{}", std::process::id()));
        let mut r = GameRecord { title: "G".into(), title_id: "PPSA00001".into(), ..Default::default() };
        add_attempt(&mut r, Attempt { engine: "kyty".into(), worked: true, ..Default::default() });
        save(&data, "PPSA00001", &r).unwrap();
        assert_eq!(load(&data, "PPSA00001"), r);
        assert_eq!(load(&data, "missing"), GameRecord::default());
        let _ = fs::remove_dir_all(data);
    }
}
