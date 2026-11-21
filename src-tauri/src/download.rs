//! Downloads the latest official release of an engine from its own GitHub repository.

use crate::logs;
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::engines::EngineDef;

#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub stage: String, // "fetching" | "downloading" | "extracting" | "done" | "error"
    pub received: u64,
    pub total: u64,
    pub message: String,
    /// Engine id this progress belongs to.
    pub engine: String,
}

pub fn asset_suffix() -> &'static str {
    if cfg!(windows) {
        "-Windows-x64.zip"
    } else if cfg!(target_os = "macos") {
        "-macOS-x86_64.zip"
    } else {
        "-Linux-x86_64.zip"
    }
}

/// Picks the right asset from a GitHub release JSON. Returns (name, url, size).
pub fn pick_asset(release: &serde_json::Value, suffix: &str) -> Option<(String, String, u64)> {
    release.get("assets")?.as_array()?.iter().find_map(|a| {
        let name = a.get("name")?.as_str()?;
        if !name.ends_with(suffix) {
            return None;
        }
        let url = a.get("browser_download_url")?.as_str()?;
        let size = a.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
        Some((name.to_string(), url.to_string(), size))
    })
}

/// Scores a release file for this PC. None = not usable (wrong system, symbols, sources…).
pub fn score_asset(name: &str) -> Option<i32> {
    let n = name.to_ascii_lowercase();
    if !n.ends_with(".zip") {
        return None;
    }
    let bad = ["pdb", "symbol", "debug", "source", "src", "-qt", "_qt", "qtlauncher", "arm64", "aarch64", "sha256"];
    if bad.iter().any(|b| n.contains(b)) {
        return None;
    }
    let (want, others): (&[&str], &[&str]) = if cfg!(windows) {
        (&["win"], &["linux", "macos", "mac-", "osx", "darwin"])
    } else if cfg!(target_os = "macos") {
        (&["mac", "osx", "darwin"], &["win", "linux"])
    } else {
        (&["linux"], &["win", "macos", "mac-", "osx", "darwin"])
    };
    if others.iter().any(|o| n.contains(o)) {
        return None;
    }
    let mut score = 0;
    if want.iter().any(|w| n.contains(w)) {
        score += 10;
    }
    if ["x64", "x86_64", "amd64"].iter().any(|a| n.contains(a)) {
        score += 3;
    }
    if n.contains("sdl") {
        score += 1;
    }
    Some(score)
}

/// Best usable file of a release, for this PC.
pub fn pick_best(release: &serde_json::Value) -> Option<(String, String, u64)> {
    let assets = release.get("assets")?.as_array()?;
    assets
        .iter()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?;
            let s = score_asset(name)?;
            let url = a.get("browser_download_url")?.as_str()?;
            let size = a.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
            Some((s, (name.to_string(), url.to_string(), size)))
        })
        .filter(|(s, _)| *s >= 10 || assets.len() == 1)
        .max_by_key(|(s, _)| *s)
        .map(|(_, a)| a)
}

/// From a list of releases: the newest stable one with a usable file, else the newest pre-release.
pub fn pick_release(list: &serde_json::Value) -> Option<(String, (String, String, u64))> {
    let rels = list.as_array()?;
    let usable = |want_pre: bool| {
        rels.iter().filter(|r| !r.get("draft").and_then(|d| d.as_bool()).unwrap_or(false)).find_map(|r| {
            let pre = r.get("prerelease").and_then(|d| d.as_bool()).unwrap_or(false);
            if pre != want_pre {
                return None;
            }
            let tag = r.get("tag_name").and_then(|t| t.as_str()).unwrap_or("latest").to_string();
            pick_best(r).map(|a| (tag, a))
        })
    };
    usable(false).or_else(|| usable(true))
}

pub fn extract(zip_path: &Path, dest: &Path) -> Result<usize, String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("bad zip: {e}"))?;
    let mut count = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name rejects "../" and absolute paths.
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        std::io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

/// Blocking. Reports progress through `report`. Returns the folder it extracted to.
pub fn download_latest(def: &EngineDef, data: &Path, report: &dyn Fn(Progress)) -> Result<PathBuf, String> {
    let id = def.id;
    let p = |stage: &str, r: u64, t: u64, m: &str| Progress { stage: stage.into(), received: r, total: t, message: m.into(), engine: id.into() };
    report(p("fetching", 0, 0, &format!("Asking GitHub for the latest {} release…", def.name)));
    let agent = ureq::AgentBuilder::new().user_agent("Strata-Launcher").build();
    let body = agent
        .get(&format!("https://api.github.com/repos/{}/releases?per_page=15", def.repo))
        .call()
        .map_err(|e| format!("GitHub request failed: {e}"))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let list: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let (tag, (name, url, size)) =
        pick_release(&list).ok_or_else(|| format!("No {} release has a .zip for this system yet.", def.name))?;
    logs::info("download", &format!("{} release {tag}: {name} ({} MB)", def.name, size / 1_048_576));

    let engine_dir = crate::engines::bin_dir(data, id);
    fs::create_dir_all(&engine_dir).map_err(|e| e.to_string())?;
    let zip_path = engine_dir.join("_download.zip");
    let resp = agent.get(&url).call().map_err(|e| format!("download failed: {e}"))?;
    let total = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(size);
    let mut reader = resp.into_reader();
    let mut file = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut received = 0u64;
    let mut last = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        received += n as u64;
        if received - last > 2 * 1024 * 1024 {
            last = received;
            report(p("downloading", received, total, &name));
        }
    }
    drop(file);
    if total > 0 && received < total {
        return Err(format!("download incomplete ({received} of {total} bytes)"));
    }

    report(p("extracting", received, total, "Unpacking…"));
    let dest = engine_dir.join(name.trim_end_matches(".zip"));
    if dest.exists() {
        let _ = fs::remove_dir_all(&dest);
    }
    let files = extract(&zip_path, &dest)?;
    let _ = fs::remove_file(&zip_path);
    // Keep only the new release next to it (saves live elsewhere, in the engine's user folder).
    if id != "kyty" {
        if let Ok(rd) = fs::read_dir(&engine_dir) {
            for e in rd.flatten() {
                if e.path() != dest && e.path().is_dir() {
                    let _ = fs::remove_dir_all(e.path());
                }
            }
        }
    }
    logs::info("download", &format!("Extracted {files} files to {}", dest.display()));
    report(p("done", received, total, &tag));
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_windows_asset() {
        let rel: serde_json::Value = serde_json::from_str(
            r#"{"tag_name":"KytyPS5-2026-11-20-abc","assets":[
                {"name":"KytyPS5-2026-11-20-abc-Linux-x86_64.zip","browser_download_url":"https://x/l","size":10},
                {"name":"KytyPS5-2026-11-20-abc-Windows-x64.zip","browser_download_url":"https://x/w","size":20}]}"#,
        )
        .unwrap();
        let (n, u, s) = pick_asset(&rel, "-Windows-x64.zip").unwrap();
        assert!(n.ends_with("Windows-x64.zip"));
        assert_eq!((u.as_str(), s), ("https://x/w", 20));
        assert!(pick_asset(&rel, "-Nope.zip").is_none());
    }

    #[test]
    fn picks_release_files_for_any_engine() {
        let win = cfg!(windows);
        let list: serde_json::Value = serde_json::from_str(
            r#"[
              {"tag_name":"draft","draft":true,"prerelease":false,"assets":[{"name":"x-win64.zip","browser_download_url":"d","size":1}]},
              {"tag_name":"nightly","draft":false,"prerelease":true,"assets":[
                {"name":"shadps4-win64-sdl-abc.zip","browser_download_url":"pw","size":1},
                {"name":"shadps4-linux-sdl-abc.zip","browser_download_url":"pl","size":1}]},
              {"tag_name":"v.0.20.0","draft":false,"prerelease":false,"assets":[
                {"name":"shadps4-win64-qt-0.20.0.zip","browser_download_url":"qt","size":1},
                {"name":"shadps4-win64-sdl-0.20.0.zip","browser_download_url":"w","size":5},
                {"name":"shadps4-linux-sdl-0.20.0.zip","browser_download_url":"l","size":6},
                {"name":"shadps4-win64-sdl-0.20.0.pdb.zip","browser_download_url":"pdb","size":7}]}
            ]"#,
        )
        .unwrap();
        let (tag, (_, url, _)) = pick_release(&list).unwrap();
        assert_eq!(tag, "v.0.20.0", "stable beats newer pre-release");
        assert_eq!(url, if win { "w" } else { "l" });

        // Only pre-releases: take the newest one.
        let pre: serde_json::Value = serde_json::from_str(
            r#"[{"tag_name":"p1","prerelease":true,"assets":[{"name":"SharpEmu-windows-x64.zip","browser_download_url":"sw","size":1},{"name":"SharpEmu-linux-x64.zip","browser_download_url":"sl","size":1}]}]"#,
        )
        .unwrap();
        assert_eq!(pick_release(&pre).unwrap().1 .1, if win { "sw" } else { "sl" });
        assert_eq!(score_asset("tool.7z"), None);
        assert_eq!(score_asset("relinker-src.zip"), None);
        assert!(pick_release(&serde_json::json!([])).is_none());
    }

    #[test]
    fn extract_is_safe() {
        let dir = std::env::temp_dir().join(format!("strata_zip_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let zp = dir.join("t.zip");
        {
            let f = fs::File::create(&zp).unwrap();
            let mut w = zip::ZipWriter::new(f);
            let o = zip::write::SimpleFileOptions::default();
            w.start_file("ok/kyty_emulator.exe", o).unwrap();
            w.write_all(b"bin").unwrap();
            w.start_file("../evil.txt", o).unwrap();
            w.write_all(b"no").unwrap();
            w.finish().unwrap();
        }
        let out = dir.join("out");
        assert_eq!(extract(&zp, &out).unwrap(), 1);
        assert!(out.join("ok/kyty_emulator.exe").is_file());
        assert!(!dir.join("evil.txt").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
