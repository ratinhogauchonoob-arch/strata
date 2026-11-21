//! Installs PS4 .pkg files (games, updates, DLC) dumped from the user's own console.
//! Only fake PKGs (FPKG) open: retail packages are encrypted for a console and are refused.
//! Extraction uses orbis-pkg / orbis-pkg-util (MIT OR Apache-2.0).

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Game,
    Update,
    Dlc,
}

#[derive(Clone, Debug, Serialize)]
pub struct PkgInfo {
    pub path: String,
    pub content_id: String,
    pub title_id: String,
    pub label: String,
    pub kind: Kind,
    pub kind_name: String,
    pub size_mb: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub file: String,
    pub stage: String, // "extracting" | "done" | "error"
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub message: String,
}

/// Content type + flags → what the package is.
pub fn classify(content_type: u32, flags: u32) -> Kind {
    const PATCH_BITS: u32 = 0x0010_0000 | 0x0100_0000 | 0x2000_0000 | 0x4000_0000;
    match content_type {
        0x02 | 0x1B => Kind::Dlc,
        0x04 | 0x05 => Kind::Update,
        _ if flags & PATCH_BITS != 0 => Kind::Update,
        _ => Kind::Game,
    }
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Game => "Game",
        Kind::Update => "Update",
        Kind::Dlc => "DLC",
    }
}

fn friendly(e: orbis_pkg::OpenError) -> String {
    use orbis_pkg::OpenError::*;
    match e {
        ReadHeaderFailed { .. } => "This is not a PS4 package (the header is wrong or the file is damaged).".into(),
        DecryptEntryKeyFailed { .. } | DecryptEkpfsFailed { .. } | EntryKeyNotFound | PfsImageKeyNotFound | GetPfsImageKeyFailed { .. } | FindPfsImageKeyFailed { .. } | FindEntryKeyFailed { .. } => {
            "This package is encrypted for a console (a store/retail PKG). Strata can only install packages you dumped from your own console (fake PKG).".into()
        }
        other => format!("Could not read the package: {other}"),
    }
}

fn open(path: &Path) -> Result<orbis_pkg::Pkg<memmap2::Mmap>, String> {
    // SAFETY: the file is opened read-only and only read while the map is alive.
    unsafe { orbis_pkg_util::open_pkg(path) }.map_err(|e| match e {
        orbis_pkg_util::OpenPkgError::ParsePkg { source } => friendly(source),
        orbis_pkg_util::OpenPkgError::OpenFile { source } | orbis_pkg_util::OpenPkgError::MmapFile { source } => {
            format!("Could not open the file ({source}). If it is still downloading or open in another program, wait and try again.")
        }
    })
}

pub fn inspect(path: &Path) -> Result<PkgInfo, String> {
    let pkg = open(path)?;
    let h = pkg.header();
    let cid = h.content_id();
    let kind = classify(h.content_type(), h.content_flags().bits());
    Ok(PkgInfo {
        path: path.to_string_lossy().into_owned(),
        content_id: cid.as_str().to_string(),
        title_id: cid.title_id().to_string(),
        label: cid.label().to_string(),
        kind,
        kind_name: kind_name(kind).into(),
        size_mb: path.metadata().map(|m| m.len() / 1_048_576).unwrap_or(0),
    })
}

/// Where a package goes. `game_dir` is the installed game with the same title id, if any.
/// shadPS4 reads updates from "<game>-UPDATE" next to the game and DLC from
/// "<user>/addcont/<TITLE_ID>/<label>".
pub fn destination(info: &PkgInfo, library: &Path, game_dir: Option<&Path>, shadps4_user: &Path) -> PathBuf {
    match info.kind {
        Kind::Game => library.join(&info.title_id),
        Kind::Update => match game_dir {
            Some(g) => PathBuf::from(format!("{}-UPDATE", g.to_string_lossy().trim_end_matches(['/', '\\']))),
            None => library.join(format!("{}-UPDATE", info.title_id)),
        },
        Kind::Dlc => {
            let label = if info.label.is_empty() { info.content_id.clone() } else { info.label.clone() };
            shadps4_user.join("addcont").join(&info.title_id).join(label)
        }
    }
}

struct Reporter<'a> {
    report: &'a (dyn Fn(Progress) + Sync),
    file: String,
    total_files: AtomicUsize,
    done_files: AtomicUsize,
    bytes: AtomicU64,
    bytes_total: u64,
    last: Mutex<Instant>,
}

impl Reporter<'_> {
    fn send(&self, force: bool) {
        let mut last = self.last.lock().unwrap();
        if !force && last.elapsed() < Duration::from_millis(250) {
            return;
        }
        *last = Instant::now();
        (self.report)(Progress {
            file: self.file.clone(),
            stage: "extracting".into(),
            files_done: self.done_files.load(Ordering::Relaxed),
            files_total: self.total_files.load(Ordering::Relaxed),
            bytes_done: self.bytes.load(Ordering::Relaxed),
            bytes_total: self.bytes_total,
            message: String::new(),
        });
    }
}

impl orbis_pkg_util::ExtractProgress for Reporter<'_> {
    fn pfs_start(&self, total_items: usize) {
        self.total_files.store(total_items, Ordering::Relaxed);
        self.send(true);
    }
    fn pfs_file_completed(&self, written: u64) {
        self.done_files.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(written, Ordering::Relaxed);
        self.send(false);
    }
}

/// Extracts into "<dest>.part" and renames it when complete, so a failed install leaves nothing half-done.
pub fn install(path: &Path, dest: &Path, overwrite: bool, report: &(dyn Fn(Progress) + Sync)) -> Result<PathBuf, String> {
    if dest.exists() {
        if !overwrite {
            return Err(format!("already-installed:{}", dest.display()));
        }
    }
    let pkg = open(path)?;
    let part = PathBuf::from(format!("{}.part", dest.to_string_lossy()));
    let _ = std::fs::remove_dir_all(&part);
    std::fs::create_dir_all(&part).map_err(|e| format!("Cannot create {}: {e}", part.display()))?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let reporter = Reporter {
        report,
        file: name.clone(),
        total_files: AtomicUsize::new(0),
        done_files: AtomicUsize::new(0),
        bytes: AtomicU64::new(0),
        bytes_total: path.metadata().map(|m| m.len()).unwrap_or(0),
        last: Mutex::new(Instant::now() - Duration::from_secs(1)),
    };
    let result = orbis_pkg_util::PkgExtractor::new(&pkg, reporter, true).extract(&part);
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&part);
        return Err(format!("Extraction failed: {e}"));
    }
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| format!("Cannot replace {}: {e}", dest.display()))?;
    }
    std::fs::rename(&part, dest).map_err(|e| format!("Cannot finish the install: {e}"))?;
    Ok(dest.to_path_buf())
}

/// What is installed next to a game: update, mods (on/off) and DLC count.
#[derive(Clone, Debug, Serialize, Default)]
pub struct Extras {
    pub update: Option<String>,
    pub update_version: String,
    pub mods: Option<String>,
    pub mods_enabled: bool,
    pub mods_files: usize,
    pub dlc: Vec<String>,
}

fn count_files(dir: &Path, depth: usize) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    rd.flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() && depth > 0 {
                count_files(&p, depth - 1)
            } else if p.is_file() {
                1
            } else {
                0
            }
        })
        .sum()
}

pub fn extras(game_dir: &Path, title_id: &str, shadps4_user: &Path) -> Extras {
    let base = game_dir.to_string_lossy().trim_end_matches(['/', '\\']).to_string();
    let mut x = Extras::default();
    for suffix in ["-UPDATE", "-patch"] {
        let p = PathBuf::from(format!("{base}{suffix}"));
        if p.is_dir() {
            x.update_version = std::fs::read(p.join("sce_sys/param.sfo")).map(|b| crate::games::parse_sfo(&b).version).unwrap_or_default();
            x.update = Some(p.to_string_lossy().into_owned());
            break;
        }
    }
    let on = PathBuf::from(format!("{base}-mods"));
    let off = PathBuf::from(format!("{base}-mods.off"));
    if on.is_dir() {
        x.mods_files = count_files(&on, 8);
        x.mods = Some(on.to_string_lossy().into_owned());
        x.mods_enabled = true;
    } else if off.is_dir() {
        x.mods_files = count_files(&off, 8);
        x.mods = Some(off.to_string_lossy().into_owned());
    }
    if !title_id.is_empty() {
        if let Ok(rd) = std::fs::read_dir(shadps4_user.join("addcont").join(title_id)) {
            x.dlc = rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            x.dlc.sort();
        }
    }
    x
}

/// Turns the mods folder on or off (shadPS4 only reads "<game>-mods"). Creates it when missing.
pub fn set_mods(game_dir: &Path, enabled: bool) -> Result<PathBuf, String> {
    let base = game_dir.to_string_lossy().trim_end_matches(['/', '\\']).to_string();
    let on = PathBuf::from(format!("{base}-mods"));
    let off = PathBuf::from(format!("{base}-mods.off"));
    let (from, to) = if enabled { (off, on) } else { (on, off) };
    if from.is_dir() && !to.exists() {
        std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
    } else if !to.exists() {
        std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
    }
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(kind: Kind) -> PkgInfo {
        PkgInfo { path: "x.pkg".into(), content_id: "UP0001-CUSA00001_00-GAMELABEL0000000".into(), title_id: "CUSA00001".into(), label: "GAMELABEL0000000".into(), kind, kind_name: String::new(), size_mb: 0 }
    }

    #[test]
    fn classification() {
        assert_eq!(classify(0x1A, 0), Kind::Game);
        assert_eq!(classify(0x1A, 0x0010_0000), Kind::Update, "first patch flag");
        assert_eq!(classify(0x1A, 0x6000_0000), Kind::Update, "cumulative patch");
        assert_eq!(classify(0x04, 0), Kind::Update);
        assert_eq!(classify(0x1B, 0), Kind::Dlc);
        assert_eq!(classify(0x02, 0x0200_0000), Kind::Dlc);
    }

    #[test]
    fn destinations_match_shadps4() {
        let lib = Path::new("/lib");
        let user = Path::new("/s/user");
        assert_eq!(destination(&info(Kind::Game), lib, None, user), Path::new("/lib/CUSA00001"));
        assert_eq!(destination(&info(Kind::Update), lib, Some(Path::new("/games/My Game")), user), Path::new("/games/My Game-UPDATE"));
        assert_eq!(destination(&info(Kind::Update), lib, None, user), Path::new("/lib/CUSA00001-UPDATE"));
        assert_eq!(destination(&info(Kind::Dlc), lib, None, user), Path::new("/s/user/addcont/CUSA00001/GAMELABEL0000000"));
    }

    #[test]
    fn junk_files_are_refused_cleanly() {
        let dir = std::env::temp_dir().join(format!("strata_pkg_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("not.pkg");
        std::fs::write(&f, vec![0u8; 4096]).unwrap();
        let e = inspect(&f).unwrap_err();
        assert!(e.contains("not a PS4 package"), "{e}");
        let dest = dir.join("out");
        assert!(install(&f, &dest, false, &|_| {}).is_err());
        assert!(!dest.exists() && !dir.join("out.part").exists(), "nothing left behind");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn extras_and_mods_toggle() {
        let root = std::env::temp_dir().join(format!("strata_extras_{}", std::process::id()));
        let game = root.join("CUSA00001");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(root.join("CUSA00001-UPDATE/sce_sys")).unwrap();
        let user = root.join("user");
        std::fs::create_dir_all(user.join("addcont/CUSA00001/DLC1")).unwrap();
        let x = extras(&game, "CUSA00001", &user);
        assert!(x.update.is_some());
        assert_eq!(x.dlc, ["DLC1"]);
        assert!(x.mods.is_none());
        let p = set_mods(&game, true).unwrap();
        assert!(p.ends_with("CUSA00001-mods") && p.is_dir());
        std::fs::write(p.join("a.txt"), b"m").unwrap();
        set_mods(&game, false).unwrap();
        let x = extras(&game, "CUSA00001", &user);
        assert!(!x.mods_enabled && x.mods_files == 1);
        set_mods(&game, true).unwrap();
        assert!(extras(&game, "CUSA00001", &user).mods_enabled);
        let _ = std::fs::remove_dir_all(root);
    }
}
