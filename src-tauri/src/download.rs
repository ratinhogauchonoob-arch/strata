//! Downloads the latest official engine release from GitHub (KytyPS5/KytyPS5, GPL-2.0).

use crate::logs;
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const REPO: &str = "KytyPS5/KytyPS5";

#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub stage: String, // "fetching" | "downloading" | "extracting" | "done" | "error"
    pub received: u64,
    pub total: u64,
    pub message: String,
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
pub fn download_latest(data: &Path, report: &dyn Fn(Progress)) -> Result<PathBuf, String> {
    let p = |stage: &str, r: u64, t: u64, m: &str| Progress { stage: stage.into(), received: r, total: t, message: m.into() };
    report(p("fetching", 0, 0, "Asking GitHub for the latest engine release…"));
    let agent = ureq::AgentBuilder::new().user_agent("Strata-Launcher").build();
    let body = agent
        .get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .call()
        .map_err(|e| format!("GitHub request failed: {e}"))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let release: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let tag = release.get("tag_name").and_then(|t| t.as_str()).unwrap_or("latest").to_string();
    let (name, url, size) =
        pick_asset(&release, asset_suffix()).ok_or_else(|| format!("release {tag} has no {} file", asset_suffix()))?;
    logs::info("download", &format!("Engine release {tag}: {name} ({} MB)", size / 1_048_576));

    let engine_dir = data.join("engine");
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
