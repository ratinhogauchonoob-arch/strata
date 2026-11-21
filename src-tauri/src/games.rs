//! Game library: finds game folders (with eboot.bin) and .zar archives, reads their metadata.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 5;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Game {
    /// Stable id: the normalized path.
    pub id: String,
    pub path: String,
    pub title: String,
    pub title_id: String,
    pub version: String,
    pub firmware: String,
    pub archive: bool,
    pub icon: Option<String>,
    pub background: Option<String>,
    pub size_mb: u64,
    pub folder: String,
}

pub fn normalize(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    s.trim_end_matches('/').to_string()
}

#[derive(Default, Debug, PartialEq)]
pub struct Meta {
    pub title: String,
    pub title_id: String,
    pub version: String,
    pub firmware: String,
}

fn jstr(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").trim().to_string()
}

/// Same rules as the engine's own launcher.
pub fn parse_param(text: &str) -> Meta {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(text) else {
        return Meta::default();
    };
    let mut m = Meta::default();
    if let Some(loc) = root.get("localizedParameters") {
        let def = jstr(loc, "defaultLanguage");
        let pick = |lang: &str| loc.get(lang).map(|o| jstr(o, "titleName")).filter(|t| !t.is_empty());
        m.title = pick(&def)
            .or_else(|| pick("en-US"))
            .or_else(|| {
                loc.as_object()
                    .and_then(|o| o.values().filter_map(|v| v.get("titleName")?.as_str()).find(|t| !t.trim().is_empty()))
                    .map(|t| t.trim().to_string())
            })
            .unwrap_or_default();
    }
    m.title_id = jstr(&root, "titleId");
    if m.title_id.is_empty() {
        let cid = jstr(&root, "contentId");
        if cid.len() == 36 && cid.as_bytes()[6] == b'-' && cid.as_bytes()[16] == b'_' {
            m.title_id = cid[7..16].to_string();
        }
    }
    m.version = jstr(&root, "appVersion");
    if m.version.is_empty() {
        m.version = jstr(&root, "contentVersion");
    }
    let fw = jstr(&root, "requiredSystemSoftwareVersion");
    if fw.len() == 18 && (fw.starts_with("0x") || fw.starts_with("0X")) {
        let d = &fw[2..8];
        if d.chars().all(|c| c.is_ascii_digit()) {
            let major: u32 = d[..2].parse().unwrap_or(0);
            m.firmware = format!("{major}.{}", &d[2..4]);
            if &d[4..6] != "00" {
                m.firmware.push('.');
                m.firmware.push_str(&d[4..6]);
            }
        }
    }
    m
}

fn dir_size_mb(dir: &Path) -> u64 {
    fn walk(p: &Path, depth: usize) -> u64 {
        if depth > 12 {
            return 0;
        }
        let Ok(rd) = fs::read_dir(p) else { return 0 };
        rd.flatten()
            .map(|e| match e.file_type() {
                Ok(t) if t.is_dir() => walk(&e.path(), depth + 1),
                Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
                _ => 0,
            })
            .sum()
    }
    walk(dir, 0) / (1024 * 1024)
}

fn read_game(dir: &Path, folder: &str, with_size: bool) -> Game {
    let sys = dir.join("sce_sys");
    let meta = fs::read_to_string(sys.join("param.json")).map(|t| parse_param(&t)).unwrap_or_default();
    let fallback = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let img = |name: &str| {
        let p = sys.join(name);
        p.is_file().then(|| p.to_string_lossy().into_owned())
    };
    Game {
        id: normalize(dir),
        path: dir.to_string_lossy().into_owned(),
        title: if meta.title.is_empty() { fallback } else { meta.title },
        title_id: meta.title_id,
        version: meta.version,
        firmware: meta.firmware,
        archive: false,
        icon: img("icon0.png"),
        background: img("pic0.png").or_else(|| img("pic1.png")),
        size_mb: if with_size { dir_size_mb(dir) } else { 0 },
        folder: folder.to_string(),
    }
}

fn scan_dir(dir: &Path, folder: &str, depth: usize, out: &mut Vec<Game>, with_size: bool) {
    if depth > MAX_DEPTH {
        return;
    }
    if dir.join("eboot.bin").is_file() {
        out.push(read_game(dir, folder, with_size));
        return; // a game folder never contains other games
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if name.starts_with('.') || name == "sce_sys" || name == "$recycle.bin" || name == "system volume information" {
            continue;
        }
        if p.is_dir() {
            scan_dir(&p, folder, depth + 1, out, with_size);
        } else if name.ends_with(".zar") {
            let size = p.metadata().map(|m| m.len() / (1024 * 1024)).unwrap_or(0);
            out.push(Game {
                id: normalize(&p),
                path: p.to_string_lossy().into_owned(),
                title: p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                title_id: String::new(),
                version: String::new(),
                firmware: String::new(),
                archive: true,
                icon: None,
                background: None,
                size_mb: size,
                folder: folder.to_string(),
            });
        }
    }
}

pub fn scan(folders: &[String], with_size: bool) -> Vec<Game> {
    let mut out = Vec::new();
    for f in folders {
        let p = Path::new(f);
        if p.is_dir() {
            scan_dir(p, f, 0, &mut out, with_size);
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out.dedup_by(|a, b| a.id == b.id);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_parsing() {
        let j = r#"{
          "localizedParameters": {"defaultLanguage":"fr-FR","fr-FR":{"titleName":"Le Jeu"},"en-US":{"titleName":"The Game"}},
          "contentId": "UP0000-PPSA01234_00-0000000000000000",
          "contentVersion": "01.000.000",
          "requiredSystemSoftwareVersion": "0x0450000000000000"
        }"#;
        let m = parse_param(j);
        assert_eq!(m.title, "Le Jeu");
        assert_eq!(m.title_id, "PPSA01234");
        assert_eq!(m.version, "01.000.000");
        assert_eq!(m.firmware, "4.50");
        assert_eq!(parse_param("not json"), Meta::default());
    }

    #[test]
    fn scans_nested_folders_and_archives() {
        let root = std::env::temp_dir().join(format!("strata_games_{}", std::process::id()));
        let game = root.join("A/MyGame");
        fs::create_dir_all(game.join("sce_sys")).unwrap();
        fs::write(game.join("eboot.bin"), b"x").unwrap();
        fs::write(game.join("sce_sys/param.json"), r#"{"titleId":"ABCD12345","localizedParameters":{"en-US":{"titleName":"Zeta"}}}"#).unwrap();
        fs::write(game.join("sce_sys/icon0.png"), b"png").unwrap();
        fs::write(root.join("Alpha.zar"), b"zar").unwrap();
        fs::create_dir_all(root.join("empty/deeper")).unwrap();

        let list = scan(&[root.to_string_lossy().into_owned()], true);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].title, "Alpha");
        assert!(list[0].archive);
        assert_eq!(list[1].title, "Zeta");
        assert_eq!(list[1].title_id, "ABCD12345");
        assert!(list[1].icon.is_some());
        assert!(list[1].background.is_none());
        let _ = fs::remove_dir_all(root);
    }
}
