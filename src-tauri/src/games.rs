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
    /// Transparent title logo (only from the user's own art folder).
    pub logo: Option<String>,
    pub size_mb: u64,
    pub folder: String,
    /// "ps5" or "ps4" (picks which engines can run it).
    pub platform: String,
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

/// Reads a PS4 param.sfo (binary key/value table).
pub fn parse_sfo(b: &[u8]) -> Meta {
    let u16_at = |o: usize| b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]]) as usize);
    let u32_at = |o: usize| b.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]));
    let mut m = Meta::default();
    if b.len() < 20 || &b[0..4] != b"\0PSF" {
        return m;
    }
    let (Some(keys), Some(vals), Some(count)) = (u32_at(8), u32_at(12), u32_at(16)) else { return m };
    let (keys, vals) = (keys as usize, vals as usize);
    for i in 0..(count as usize).min(256) {
        let e = 20 + i * 16;
        let (Some(ko), Some(fmt), Some(len), Some(doff)) = (u16_at(e), u16_at(e + 2), u32_at(e + 4), u32_at(e + 12)) else { break };
        let Some(key_bytes) = b.get(keys + ko..) else { continue };
        let key = String::from_utf8_lossy(key_bytes.split(|&c| c == 0).next().unwrap_or_default()).into_owned();
        let start = vals + doff as usize;
        let Some(raw) = b.get(start..start + len as usize) else { continue };
        if fmt == 0x0404 {
            if key == "SYSTEM_VER" && raw.len() >= 4 {
                let v = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
                let hex = format!("{v:08x}");
                let major: u32 = hex[0..2].parse().unwrap_or(0);
                m.firmware = format!("{major}.{}", &hex[2..4]);
            }
            continue;
        }
        let text = String::from_utf8_lossy(raw.split(|&c| c == 0).next().unwrap_or_default()).trim().to_string();
        match key.as_str() {
            "TITLE" => m.title = text,
            "TITLE_ID" => m.title_id = text,
            "APP_VER" => m.version = text,
            "VERSION" if m.version.is_empty() => m.version = text,
            _ => {}
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

/// User art overrides: <game>/strata/cover.png, background.png (or .jpg), logo.png.
fn custom_art(dir: &Path, name: &str) -> Option<String> {
    ["png", "jpg", "jpeg", "webp"].iter().map(|ext| dir.join("strata").join(format!("{name}.{ext}"))).find(|p| p.is_file()).map(|p| p.to_string_lossy().into_owned())
}

fn read_game(dir: &Path, folder: &str, with_size: bool) -> Game {
    let sys = dir.join("sce_sys");
    let json = sys.join("param.json");
    let sfo = sys.join("param.sfo");
    // PS5 dumps carry param.json; PS4 dumps carry param.sfo.
    let ps4 = !json.is_file() && sfo.is_file();
    let meta = if ps4 {
        fs::read(&sfo).map(|b| parse_sfo(&b)).unwrap_or_default()
    } else {
        fs::read_to_string(&json).map(|t| parse_param(&t)).unwrap_or_default()
    };
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
        icon: custom_art(dir, "cover").or_else(|| img("icon0.png")),
        background: custom_art(dir, "background").or_else(|| img("pic0.png")).or_else(|| img("pic1.png")),
        logo: custom_art(dir, "logo"),
        size_mb: if with_size { dir_size_mb(dir) } else { 0 },
        folder: folder.to_string(),
        platform: if ps4 { "ps4" } else { "ps5" }.into(),
    }
}

/// A folder that looked like a game but was left out, and why.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Skipped {
    pub path: String,
    pub reason: String,
}

/// A real game dump has its info file next to eboot.bin; tools and payloads do not.
fn is_game_dir(dir: &Path) -> bool {
    let sys = dir.join("sce_sys");
    sys.join("param.json").is_file() || sys.join("param.sfo").is_file()
}

fn scan_dir(dir: &Path, folder: &str, depth: usize, out: &mut Vec<Game>, skipped: &mut Vec<Skipped>, with_size: bool) {
    if depth > MAX_DEPTH {
        return;
    }
    if dir.join("eboot.bin").is_file() {
        if is_game_dir(dir) {
            out.push(read_game(dir, folder, with_size));
        } else {
            skipped.push(Skipped {
                path: dir.to_string_lossy().into_owned(),
                reason: "has eboot.bin but no sce_sys/param.json or param.sfo (not a game dump)".into(),
            });
        }
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
            scan_dir(&p, folder, depth + 1, out, skipped, with_size);
        } else if name.ends_with(".zar") {
            let size = p.metadata().map(|m| m.len() / (1024 * 1024)).unwrap_or(0);
            // Art for archives sits next to them: Game.zar + Game.png / Game.background.png / Game.logo.png
            let side = |suffix: &str| {
                let stem = p.with_extension("");
                ["png", "jpg"].iter().map(|e| PathBuf::from(format!("{}{suffix}.{e}", stem.display()))).find(|x| x.is_file()).map(|x| x.to_string_lossy().into_owned())
            };
            out.push(Game {
                id: normalize(&p),
                path: p.to_string_lossy().into_owned(),
                title: p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                title_id: String::new(),
                version: String::new(),
                firmware: String::new(),
                archive: true,
                icon: side(""),
                background: side(".background"),
                logo: side(".logo"),
                size_mb: size,
                folder: folder.to_string(),
                platform: "ps5".into(),
            });
        }
    }
}

pub fn scan(folders: &[String], with_size: bool) -> Vec<Game> {
    scan_report(folders, with_size).0
}

pub fn scan_report(folders: &[String], with_size: bool) -> (Vec<Game>, Vec<Skipped>) {
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    for f in folders {
        let p = Path::new(f);
        if p.is_dir() {
            scan_dir(p, f, 0, &mut out, &mut skipped, with_size);
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out.dedup_by(|a, b| a.id == b.id);
    (out, skipped)
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
        assert_eq!(list[1].platform, "ps5");
        assert!(list[1].icon.is_some());
        assert!(list[1].background.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn tools_with_eboot_are_not_games() {
        let root = std::env::temp_dir().join(format!("strata_tool_{}", std::process::id()));
        let tool = root.join("backport (4.xx to 10.xx)");
        fs::create_dir_all(&tool).unwrap();
        fs::write(tool.join("eboot.bin"), b"x").unwrap();
        let (games, skipped) = scan_report(&[root.to_string_lossy().into_owned()], false);
        assert!(games.is_empty());
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].path.contains("backport"));
        let _ = fs::remove_dir_all(root);
    }

    fn sfo(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut keys = Vec::new();
        let mut vals = Vec::new();
        let mut index = Vec::new();
        for (k, v) in entries {
            let ko = keys.len() as u16;
            keys.extend_from_slice(k.as_bytes());
            keys.push(0);
            let doff = vals.len() as u32;
            let mut data = v.as_bytes().to_vec();
            data.push(0);
            let len = data.len() as u32;
            vals.extend_from_slice(&data);
            index.extend_from_slice(&ko.to_le_bytes());
            index.extend_from_slice(&0x0204u16.to_le_bytes());
            index.extend_from_slice(&len.to_le_bytes());
            index.extend_from_slice(&len.to_le_bytes());
            index.extend_from_slice(&doff.to_le_bytes());
        }
        // one integer entry: SYSTEM_VER 0x05050000
        let ko = keys.len() as u16;
        keys.extend_from_slice(b"SYSTEM_VER\0");
        let doff = vals.len() as u32;
        vals.extend_from_slice(&0x0505_0000u32.to_le_bytes());
        index.extend_from_slice(&ko.to_le_bytes());
        index.extend_from_slice(&0x0404u16.to_le_bytes());
        index.extend_from_slice(&4u32.to_le_bytes());
        index.extend_from_slice(&4u32.to_le_bytes());
        index.extend_from_slice(&doff.to_le_bytes());
        let n = entries.len() as u32 + 1;
        let key_start = 20 + index.len() as u32;
        let val_start = key_start + keys.len() as u32;
        let mut out = b"\0PSF".to_vec();
        out.extend_from_slice(&0x0101u32.to_le_bytes());
        out.extend_from_slice(&key_start.to_le_bytes());
        out.extend_from_slice(&val_start.to_le_bytes());
        out.extend_from_slice(&n.to_le_bytes());
        out.extend(index);
        out.extend(keys);
        out.extend(vals);
        out
    }

    #[test]
    fn sfo_parsing_and_ps4_detection() {
        let b = sfo(&[("APP_VER", "01.05"), ("TITLE", "Old Game"), ("TITLE_ID", "CUSA00001")]);
        let m = parse_sfo(&b);
        assert_eq!((m.title.as_str(), m.title_id.as_str(), m.version.as_str(), m.firmware.as_str()), ("Old Game", "CUSA00001", "01.05", "5.05"));
        assert_eq!(parse_sfo(b"junk"), Meta::default());
        assert_eq!(parse_sfo(&b[..30]).title, "", "truncated file is ignored");

        let root = std::env::temp_dir().join(format!("strata_ps4_{}", std::process::id()));
        let g = root.join("CUSA00001");
        fs::create_dir_all(g.join("sce_sys")).unwrap();
        fs::write(g.join("eboot.bin"), b"x").unwrap();
        fs::write(g.join("sce_sys/param.sfo"), &b).unwrap();
        let list = scan(&[root.to_string_lossy().into_owned()], false);
        assert_eq!(list[0].platform, "ps4");
        assert_eq!(list[0].title, "Old Game");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn custom_art_overrides() {
        let root = std::env::temp_dir().join(format!("strata_art_{}", std::process::id()));
        let game = root.join("G");
        fs::create_dir_all(game.join("sce_sys")).unwrap();
        fs::create_dir_all(game.join("strata")).unwrap();
        fs::write(game.join("eboot.bin"), b"x").unwrap();
        fs::write(game.join("sce_sys/param.json"), "{}").unwrap();
        fs::write(game.join("sce_sys/icon0.png"), b"a").unwrap();
        fs::write(game.join("strata/cover.jpg"), b"b").unwrap();
        fs::write(game.join("strata/logo.png"), b"c").unwrap();
        fs::write(root.join("Z.zar"), b"z").unwrap();
        fs::write(root.join("Z.png"), b"p").unwrap();
        fs::write(root.join("Z.logo.png"), b"l").unwrap();
        let list = scan(&[root.to_string_lossy().into_owned()], false);
        let g = list.iter().find(|x| !x.archive).unwrap();
        assert!(g.icon.as_ref().unwrap().ends_with("cover.jpg"));
        assert!(g.logo.as_ref().unwrap().ends_with("logo.png"));
        let z = list.iter().find(|x| x.archive).unwrap();
        assert!(z.icon.as_ref().unwrap().ends_with("Z.png"));
        assert!(z.logo.as_ref().unwrap().ends_with("Z.logo.png"));
        assert!(z.background.is_none());
        let _ = fs::remove_dir_all(root);
    }
}
