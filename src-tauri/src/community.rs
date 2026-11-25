//! Community results (opt-in): after a game closes, Strata can send an anonymous test result to
//! the website, and it downloads everyone's combined results to pick better engines.
//! Everything runs on background threads, so the app never waits on the network.

use crate::compat::{Attempt, EngineKnowledge, GameRecord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SITE: &str = "https://strata.gbxstudios.com";

/// What is sent. Deliberately small: no user name, paths, logs or hardware IDs.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Report {
    pub title: String,
    pub title_id: String,
    pub game_version: String,
    pub platform: String,
    pub game_engine: String,
    pub engine: String,
    pub engine_release: String,
    pub worked: bool,
    pub seconds: u64,
    pub exit_code: Option<i32>,
    pub missing_functions: Vec<String>,
    pub missing_modules: Vec<String>,
    /// Why the engine stopped (its own fatal message), with paths and quoted text removed.
    pub error: String,
    pub gpu_vendor: String,
    pub strata_version: String,
}

/// The engine's reason, made safe to share: paths are already shortened by compat; here quoted
/// text (file names, messages with user data) is dropped and only plain ASCII is kept.
pub fn share_safe_error(reason: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    for c in reason.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => {
                quote = Some(c);
                out.push_str("'…'");
            }
            None if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            None if c == '·' => out.push('|'),
            None => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(160).collect()
}

fn valid_title_id(id: &str) -> bool {
    // is_ascii first: slicing at byte 4 would panic inside a multi-byte character.
    id.len() == 9 && id.is_ascii() && id[..4].bytes().all(|b| b.is_ascii_uppercase()) && id[4..].bytes().all(|b| b.is_ascii_digit())
}

/// Folder names can hold a user name ("C:\Users\Bob\..."); only plain release names are sent.
fn clean_release(r: &str) -> String {
    if r.chars().all(|c| c.is_ascii_alphanumeric() || "._- ".contains(c)) {
        r.chars().take(80).collect()
    } else {
        String::new()
    }
}

pub fn gpu_vendor(name: &str) -> String {
    let n = name.to_ascii_lowercase();
    if n.contains("nvidia") || n.contains("geforce") || n.contains("rtx") || n.contains("gtx") {
        "NVIDIA"
    } else if n.contains("amd") || n.contains("radeon") {
        "AMD"
    } else if n.contains("intel") {
        "Intel"
    } else {
        "Other"
    }
    .into()
}

/// Builds the report for one attempt, or None when the game has no proper title ID (nothing is sent).
pub fn build(game: &crate::games::Game, rec: &GameRecord, a: &Attempt, gpu: &str) -> Option<Report> {
    if !valid_title_id(&game.title_id) {
        return None;
    }
    Some(Report {
        title: game.title.chars().filter(|c| !c.is_control()).take(100).collect(),
        title_id: game.title_id.clone(),
        game_version: game.version.chars().take(20).collect(),
        platform: game.platform.clone(),
        game_engine: rec.analysis.game_engine.clone(),
        engine: a.engine.clone(),
        engine_release: clean_release(&a.release),
        worked: a.worked,
        seconds: a.seconds,
        exit_code: a.code,
        missing_functions: a.findings.missing_functions.iter().take(50).cloned().collect(),
        missing_modules: a.findings.missing_modules.iter().take(30).cloned().collect(),
        error: if a.worked { String::new() } else { share_safe_error(&a.findings.reason) },
        gpu_vendor: gpu_vendor(gpu),
        strata_version: crate::APP_VERSION.into(),
    })
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().user_agent(&format!("Strata/{}", crate::APP_VERSION)).timeout(std::time::Duration::from_secs(10)).build()
}

/// Sends a report on a background thread; failures are only logged.
pub fn send(r: Report) {
    std::thread::spawn(move || {
        let body = serde_json::to_string(&r).unwrap_or_default();
        match agent().post(&format!("{SITE}/api/compat/report")).set("Content-Type", "application/json").set("Accept", "application/json").send_string(&body) {
            Ok(_) => crate::logs::info("community", &format!("Shared result: {} on {} ({})", r.title, r.engine, if r.worked { "worked" } else { "failed" })),
            Err(e) => crate::logs::debug("community", &format!("Could not share result: {e}")),
        }
    });
}

// ───────────────────────── combined results from everyone ─────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Knowledge {
    pub version: u32,
    /// title id → engine → number of players it worked for
    pub worked: BTreeMap<String, BTreeMap<String, u32>>,
    /// engine → release → functions seen missing
    pub missing_functions: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// When Strata downloaded it (unix seconds).
    pub fetched: u64,
}

pub fn knowledge_file(data: &Path) -> PathBuf {
    crate::compat::dir(data).join("community.json")
}

pub fn load(data: &Path) -> Knowledge {
    std::fs::read_to_string(knowledge_file(data)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Downloads the combined results at most every 12 hours (background thread).
pub fn refresh(data: PathBuf) {
    std::thread::spawn(move || {
        if now().saturating_sub(load(&data).fetched) < 12 * 3600 {
            return;
        }
        let res = agent().get(&format!("{SITE}/api/compat/knowledge")).set("Accept", "application/json").call();
        match res.map_err(|e| e.to_string()).and_then(|r| r.into_string().map_err(|e| e.to_string())) {
            Ok(text) => match serde_json::from_str::<Knowledge>(&text) {
                Ok(mut k) => {
                    k.fetched = now();
                    let _ = std::fs::create_dir_all(crate::compat::dir(&data));
                    let _ = std::fs::write(knowledge_file(&data), serde_json::to_vec(&k).unwrap_or_default());
                    crate::logs::info("community", &format!("Community results updated: {} game(s)", k.worked.len()));
                }
                Err(e) => crate::logs::debug("community", &format!("Community results unreadable: {e}")),
            },
            Err(e) => crate::logs::debug("community", &format!("Community results not downloaded: {e}")),
        }
    });
}

/// Adds the community's missing functions to the local engine knowledge.
pub fn merge_into(local: &mut EngineKnowledge, k: &Knowledge) {
    for (engine, releases) in &k.missing_functions {
        for (release, nids) in releases {
            local.missing_functions.entry(engine.clone()).or_default().entry(release.clone()).or_default().extend(nids.iter().cloned());
        }
    }
}

/// Engines that worked for other players on this game, as fake "worked" attempts on any release
/// (scored like "an earlier version of this engine ran it").
pub fn worked_for_others(k: &Knowledge, title_id: &str) -> Vec<(String, u32)> {
    k.worked.get(title_id).map(|m| m.iter().map(|(e, n)| (e.clone(), *n)).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compat::Findings;

    #[test]
    fn odd_title_ids_never_panic() {
        assert!(valid_title_id("PPSA17221"));
        assert!(!valid_title_id("PPSÁ1722"), "9 bytes with an accent");
        assert!(!valid_title_id("ppsa17221"));
        assert!(!valid_title_id(""));
    }

    fn game(id: &str) -> crate::games::Game {
        crate::games::Game {
            id: "D:/Games/G".into(),
            path: "C:/Users/Bob/Games/G".into(),
            title: "Goat\u{7} Sim".into(),
            title_id: id.into(),
            version: "01.000".into(),
            firmware: String::new(),
            archive: false,
            icon: None,
            background: None,
            logo: None,
            size_mb: 0,
            folder: String::new(),
            platform: "ps5".into(),
        }
    }

    #[test]
    fn report_has_nothing_personal() {
        let a = Attempt {
            engine: "sharpemu".into(),
            release: "sharpemu-0.0.7-pulse-win-x64".into(),
            seconds: 6,
            code: Some(4),
            worked: false,
            findings: Findings { missing_functions: ["qCSfqDILlns".to_string()].into(), last_error: "C:\\Users\\Bob crashed".into(), ..Default::default() },
            ..Default::default()
        };
        let r = build(&game("PPSA04159"), &GameRecord::default(), &a, "NVIDIA GeForce RTX 4050 Laptop GPU").unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("Bob") && !json.contains("Users"), "{json}");
        assert_eq!(r.title, "Goat Sim", "control characters removed");
        assert_eq!(r.gpu_vendor, "NVIDIA");
        assert_eq!(r.missing_functions, ["qCSfqDILlns"]);
        // A release folder with a path in it is not sent.
        let a2 = Attempt { release: "C:\\Users\\Bob\\kyty".into(), ..a.clone() };
        assert_eq!(build(&game("PPSA04159"), &GameRecord::default(), &a2, "").unwrap().engine_release, "");
        // The engine's reason goes along, minus quoted text.
        let a3 = Attempt {
            findings: Findings { reason: crate::compat::shorten_paths("Unhandled exception. System.IO.FileNotFoundException: Could not find file 'C:\\Users\\Bob\\x.bin'."), ..Default::default() },
            ..a.clone()
        };
        let e = build(&game("PPSA04159"), &GameRecord::default(), &a3, "").unwrap().error;
        assert!(!e.contains("Bob") && !e.contains("x.bin") && e.contains("FileNotFoundException"), "{e}");
        assert_eq!(share_safe_error("Not implemented (a != b) · Texture.cpp:812"), "Not implemented (a != b) | Texture.cpp:812");
        // Games without a real title ID are never sent.
        assert!(build(&game(""), &GameRecord::default(), &a, "").is_none());
        assert!(build(&game("PATH-1234"), &GameRecord::default(), &a, "").is_none());
    }

    #[test]
    fn community_knowledge_merges() {
        let k: Knowledge = serde_json::from_str(r#"{"version":1,"worked":{"PPSA04159":{"kyty":3}},"missing_functions":{"sharpemu":{"r1":["qCSfqDILlns"]}}}"#).unwrap();
        let mut local = EngineKnowledge::default();
        merge_into(&mut local, &k);
        assert!(local.missing_functions["sharpemu"]["r1"].contains("qCSfqDILlns"));
        assert_eq!(worked_for_others(&k, "PPSA04159"), [("kyty".to_string(), 3)]);
        assert!(worked_for_others(&k, "PPSA00000").is_empty());
        assert_eq!(gpu_vendor("AMD Radeon RX 7600"), "AMD");
        assert_eq!(gpu_vendor("Intel(R) UHD Graphics 630"), "Intel");
    }
}
