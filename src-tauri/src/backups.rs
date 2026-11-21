//! Save and shader-cache backups. A backup is a zip with a `strata-backup.json` manifest that
//! records where every folder came from, so it can be put back exactly.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Source {
    /// Prefix inside the zip ("0", "1", ...).
    pub key: String,
    /// Absolute folder on disk.
    pub path: String,
    /// "saves" | "shaders"
    pub kind: String,
    pub engine: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub strata: String,
    pub created: String,
    pub title_id: String,
    pub sources: Vec<Source>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BackupInfo {
    pub file: String,
    pub name: String,
    pub size_kb: u64,
    pub created: String,
    pub title_id: String,
    pub folders: usize,
}

pub fn backups_dir(data: &Path) -> PathBuf {
    data.join("backups")
}

fn classify(name: &str) -> Option<&'static str> {
    let n = name.to_ascii_lowercase();
    if n.contains("save") {
        Some("saves")
    } else if n.contains("shader") || n.contains("pipeline_cache") || n == "cache" {
        Some("shaders")
    } else {
        None
    }
}

/// Finds save and shader folders under an engine's user folders (up to 3 levels deep).
/// With a title id, keeps only the parts that belong to that game when the folder is split per game.
pub fn find_sources(roots: &[(String, PathBuf)], title_id: &str, include_shaders: bool) -> Vec<Source> {
    fn walk(dir: &Path, depth: usize, engine: &str, title_id: &str, shaders: bool, out: &mut Vec<Source>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        dirs.sort();
        for d in dirs {
            let name = d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match classify(&name) {
                Some(kind) if kind == "saves" || shaders => {
                    let picked = per_game(&d, title_id).unwrap_or_else(|| vec![d.clone()]);
                    for p in picked {
                        out.push(Source { key: String::new(), path: p.to_string_lossy().into_owned(), kind: kind.into(), engine: engine.into() });
                    }
                }
                Some(_) => {}
                None if depth > 0 => walk(&d, depth - 1, engine, title_id, shaders, out),
                None => {}
            }
        }
    }
    let mut out = Vec::new();
    for (engine, root) in roots {
        walk(root, 3, engine, title_id, include_shaders, &mut out);
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.dedup_by(|a, b| a.path.starts_with(&b.path));
    for (i, s) in out.iter_mut().enumerate() {
        s.key = i.to_string();
    }
    out
}

/// Inside a save folder, the sub-folders named after the game (searched 2 levels down).
fn per_game(dir: &Path, title_id: &str) -> Option<Vec<PathBuf>> {
    if title_id.is_empty() {
        return None;
    }
    let mut hits = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 2usize)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_ascii_uppercase();
            if name.starts_with(&title_id.to_ascii_uppercase()) {
                hits.push(p);
            } else if depth > 0 {
                stack.push((p, depth - 1));
            }
        }
    }
    (!hits.is_empty()).then_some(hits)
}

fn add_dir(w: &mut zip::ZipWriter<fs::File>, base: &Path, dir: &Path, prefix: &str, count: &mut usize) -> Result<(), String> {
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let rd = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for e in rd.flatten() {
        let p = e.path();
        let rel = p.strip_prefix(base).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
        if p.is_dir() {
            add_dir(w, base, &p, prefix, count)?;
        } else if p.is_file() {
            w.start_file(format!("{prefix}/{rel}"), opts).map_err(|e| e.to_string())?;
            let mut f = fs::File::open(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            w.write_all(&buf).map_err(|e| e.to_string())?;
            *count += 1;
        }
    }
    Ok(())
}

/// Creates a backup. Returns its path and the number of files saved.
pub fn create(data: &Path, label: &str, title_id: &str, sources: &[Source], stamp: &str) -> Result<(PathBuf, usize), String> {
    if sources.is_empty() {
        return Err("No save or shader folders were found yet. Play a game once, then try again.".into());
    }
    let dir = backups_dir(data);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe: String = label.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(40).collect();
    let path = dir.join(format!("{stamp}-{safe}.zip"));
    let file = fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut w = zip::ZipWriter::new(file);
    let manifest = Manifest { strata: crate::APP_VERSION.into(), created: stamp.into(), title_id: title_id.into(), sources: sources.to_vec() };
    let opts = zip::write::SimpleFileOptions::default();
    w.start_file("strata-backup.json", opts).map_err(|e| e.to_string())?;
    w.write_all(serde_json::to_string_pretty(&manifest).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())?;
    let mut count = 0;
    for s in sources {
        let p = Path::new(&s.path);
        if p.is_dir() {
            add_dir(&mut w, p, p, &s.key, &mut count)?;
        }
    }
    w.finish().map_err(|e| e.to_string())?;
    Ok((path, count))
}

fn read_manifest(zip_path: &Path) -> Result<Manifest, String> {
    let f = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut a = zip::ZipArchive::new(f).map_err(|e| format!("not a backup: {e}"))?;
    let mut m = a.by_name("strata-backup.json").map_err(|_| "not a Strata backup (no manifest)".to_string())?;
    let mut text = String::new();
    m.read_to_string(&mut text).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("broken manifest: {e}"))
}

pub fn list(data: &Path) -> Vec<BackupInfo> {
    let Ok(rd) = fs::read_dir(backups_dir(data)) else { return vec![] };
    let mut v: Vec<BackupInfo> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "zip"))
        .filter_map(|p| {
            let m = read_manifest(&p).ok()?;
            Some(BackupInfo {
                file: p.to_string_lossy().into_owned(),
                name: p.file_stem()?.to_string_lossy().into_owned(),
                size_kb: p.metadata().map(|m| m.len() / 1024).unwrap_or(0),
                created: m.created,
                title_id: m.title_id,
                folders: m.sources.len(),
            })
        })
        .collect();
    v.sort_by(|a, b| b.name.cmp(&a.name));
    v
}

/// Puts every file back where it came from. Current files are moved aside first
/// ("<folder>.before-restore"), so a restore can itself be undone by hand.
pub fn restore(zip_path: &Path) -> Result<usize, String> {
    let m = read_manifest(zip_path)?;
    let f = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut a = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    for s in &m.sources {
        let p = Path::new(&s.path);
        if p.is_dir() {
            let aside = PathBuf::from(format!("{}.before-restore", s.path));
            let _ = fs::remove_dir_all(&aside);
            fs::rename(p, &aside).map_err(|e| format!("could not move {} aside: {e}", p.display()))?;
        }
    }
    let mut count = 0;
    for i in 0..a.len() {
        let mut entry = a.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name() else { continue };
        let mut parts = rel.components();
        let Some(first) = parts.next() else { continue };
        let key = first.as_os_str().to_string_lossy().into_owned();
        let Some(src) = m.sources.iter().find(|s| s.key == key) else { continue };
        let rest: PathBuf = parts.collect();
        if rest.as_os_str().is_empty() || entry.is_dir() {
            continue;
        }
        let out = Path::new(&src.path).join(rest);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut o = fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        std::io::copy(&mut entry, &mut o).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_and_restore_roundtrip() {
        let root = std::env::temp_dir().join(format!("strata_bak_{}", std::process::id()));
        let user = root.join("engine/user");
        fs::create_dir_all(user.join("savedata/1/CUSA00001/slot")).unwrap();
        fs::create_dir_all(user.join("savedata/1/CUSA00002")).unwrap();
        fs::create_dir_all(user.join("shader/cache")).unwrap();
        fs::create_dir_all(user.join("log")).unwrap();
        fs::write(user.join("savedata/1/CUSA00001/slot/a.dat"), b"save-a").unwrap();
        fs::write(user.join("savedata/1/CUSA00002/b.dat"), b"save-b").unwrap();
        fs::write(user.join("shader/cache/x.bin"), b"sh").unwrap();
        fs::write(user.join("log/l.txt"), b"log").unwrap();
        let roots = vec![("shadps4".to_string(), user.clone())];

        // Whole engine: saves + shaders, no logs.
        let all = find_sources(&roots, "", true);
        assert_eq!(all.iter().map(|s| s.kind.as_str()).collect::<Vec<_>>(), ["saves", "shaders"]);
        // One game: only its own save folder, no shaders.
        let one = find_sources(&roots, "CUSA00001", false);
        assert_eq!(one.len(), 1);
        assert!(one[0].path.ends_with("CUSA00001"));

        let data = root.join("data");
        let (zip, n) = create(&data, "Test Game", "CUSA00001", &one, "20261121-1310").unwrap();
        assert_eq!(n, 1);
        assert_eq!(list(&data).len(), 1);
        assert_eq!(list(&data)[0].title_id, "CUSA00001");

        fs::write(user.join("savedata/1/CUSA00001/slot/a.dat"), b"broken").unwrap();
        assert_eq!(restore(&zip).unwrap(), 1);
        assert_eq!(fs::read(user.join("savedata/1/CUSA00001/slot/a.dat")).unwrap(), b"save-a");
        assert!(user.join("savedata/1/CUSA00001.before-restore/slot/a.dat").is_file());
        assert_eq!(fs::read(user.join("savedata/1/CUSA00002/b.dat")).unwrap(), b"save-b", "other games untouched");

        assert!(create(&data, "x", "", &[], "s").is_err());
        let _ = fs::remove_dir_all(root);
    }
}
