//! Logging: a ring buffer the UI reads, a daily log file, and live events for the dev console.

use serde::Serialize;
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

const RING: usize = 5000;

#[derive(Clone, Debug, Serialize)]
pub struct LogLine {
    pub seq: u64,
    pub t: String,
    pub level: &'static str, // trace | debug | info | warn | error
    pub source: String,      // app | engine | <module>
    pub msg: String,
}

struct Logger {
    lines: VecDeque<LogLine>,
    seq: u64,
    file: Option<File>,
    min_level: u8,
    app: Option<AppHandle>,
}

static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

fn logger() -> &'static Mutex<Logger> {
    LOGGER.get_or_init(|| Mutex::new(Logger { lines: VecDeque::new(), seq: 0, file: None, min_level: 2, app: None }))
}

fn rank(level: &str) -> u8 {
    match level {
        "trace" => 0,
        "debug" => 1,
        "info" => 2,
        "warn" => 3,
        _ => 4,
    }
}

pub fn now_string() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    now.format(time::macros::format_description!("[hour]:[minute]:[second].[subsecond digits:3]")).unwrap_or_default()
}

pub fn stamp() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    now.format(time::macros::format_description!("[year][month][day]-[hour][minute][second]")).unwrap_or_default()
}

pub fn logs_dir(data: &Path) -> PathBuf {
    data.join("logs")
}

pub fn init(data: &Path, app: Option<AppHandle>, level: &str) {
    let dir = logs_dir(data);
    let _ = fs::create_dir_all(&dir);
    prune_daily(&dir, 14);
    let day = &stamp()[..8];
    let file = OpenOptions::new().create(true).append(true).open(dir.join(format!("strata-{day}.log"))).ok();
    let mut l = logger().lock().unwrap();
    l.file = file;
    l.app = app;
    l.min_level = rank(level).min(2);
}

pub fn set_level(level: &str) {
    logger().lock().unwrap().min_level = rank(level).min(2);
}

/// Removes terminal color codes (ESC[36m …) that engines print; they show up as boxes in the window.
pub fn strip_ansi(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('\u{1b}') {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            if it.peek() == Some(&'[') {
                it.next();
                // parameters and intermediates, then one final byte in @..~
                while let Some(&n) = it.peek() {
                    it.next();
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    std::borrow::Cow::Owned(out)
}

pub fn log(level: &'static str, source: &str, msg: &str) {
    let msg = strip_ansi(msg);
    let msg = msg.as_ref();
    let mut l = logger().lock().unwrap();
    // Warnings and errors are always kept; info is the default floor.
    if rank(level) < l.min_level {
        return;
    }
    l.seq += 1;
    let line = LogLine { seq: l.seq, t: now_string(), level, source: source.to_string(), msg: msg.trim_end().to_string() };
    if let Some(f) = l.file.as_mut() {
        let _ = writeln!(f, "{} {:5} [{}] {}", line.t, line.level, line.source, line.msg);
    }
    if let Some(app) = l.app.as_ref() {
        let _ = app.emit("log", &line);
    }
    l.lines.push_back(line);
    while l.lines.len() > RING {
        l.lines.pop_front();
    }
}

pub fn info(source: &str, msg: &str) {
    log("info", source, msg);
}
pub fn warn(source: &str, msg: &str) {
    log("warn", source, msg);
}
pub fn error(source: &str, msg: &str) {
    log("error", source, msg);
}
pub fn debug(source: &str, msg: &str) {
    log("debug", source, msg);
}

/// A session log file with a size cap. Engines with console output on can print thousands of
/// lines a second, which grew one session's file to several GB.
pub struct CappedFile {
    file: Option<File>,
    written: u64,
    cap: u64,
    cut: bool,
}

impl CappedFile {
    pub fn create(path: &Path, cap: u64) -> Self {
        Self { file: File::create(path).ok(), written: 0, cap, cut: false }
    }

    pub fn line(&mut self, s: &str) {
        if self.cut {
            return;
        }
        let Some(f) = self.file.as_mut() else { return };
        if self.written + s.len() as u64 + 1 > self.cap {
            self.cut = true;
            let _ = writeln!(f, "\n# … log stopped at {} MB to save disk space; the game kept running.", self.cap >> 20);
            return;
        }
        self.written += s.len() as u64 + 1;
        let _ = writeln!(f, "{s}");
    }

    /// Written even after the cap (header and exit lines).
    pub fn always(&mut self, s: &str) {
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "{s}");
        }
    }
}

/// Caps how many engine lines per second reach the app log and the window. Engines can print
/// thousands of lines a second while a game runs; every line still goes to the session file,
/// but sending them all to the UI filled the window's memory (WebView "Out of Memory").
pub struct Throttle {
    window: std::time::Instant,
    shown: u32,
    dropped: u64,
    per_sec: u32,
}

impl Throttle {
    pub fn new(per_sec: u32) -> Self {
        Self { window: std::time::Instant::now(), shown: 0, dropped: 0, per_sec }
    }

    /// True if this line may be shown. When a new second starts, returns how many were skipped.
    pub fn allow(&mut self) -> (bool, Option<u64>) {
        let mut report = None;
        if self.window.elapsed() >= std::time::Duration::from_secs(1) {
            if self.dropped > 0 {
                report = Some(self.dropped);
            }
            self.window = std::time::Instant::now();
            self.shown = 0;
            self.dropped = 0;
        }
        if self.shown < self.per_sec {
            self.shown += 1;
            (true, report)
        } else {
            self.dropped += 1;
            (false, report)
        }
    }
}

/// Guess a level for a raw engine output line.
pub fn classify(line: &str) -> &'static str {
    let l = line.to_ascii_lowercase();
    if l.contains("error") || l.contains("fatal") || l.contains("exception") || l.contains("crash") {
        "error"
    } else if l.contains("warn") || l.contains("unimplemented") || l.contains("not implemented") {
        "warn"
    } else {
        "info"
    }
}

pub fn since(seq: u64) -> Vec<LogLine> {
    logger().lock().unwrap().lines.iter().filter(|l| l.seq > seq).cloned().collect()
}

pub fn clear() {
    logger().lock().unwrap().lines.clear();
}

/// Keep only the newest `keep` daily app logs (strata-YYYYMMDD.log); they were never removed.
pub fn prune_daily(dir: &Path, keep: usize) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut days: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("strata-") && n.ends_with(".log")))
        .collect();
    days.sort();
    if days.len() > keep {
        for p in &days[..days.len() - keep] {
            let _ = fs::remove_file(p);
        }
    }
}

/// Keep only the newest `keep` session logs.
pub fn prune_sessions(data: &Path, keep: usize) {
    let Ok(rd) = fs::read_dir(logs_dir(data)) else { return };
    let mut sessions: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("session-")))
        .collect();
    sessions.sort();
    if sessions.len() > keep {
        for p in &sessions[..sessions.len() - keep] {
            let _ = fs::remove_file(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_daily_logs_are_removed() {
        let dir = std::env::temp_dir().join(format!("strata_daily_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for d in 1..=5 {
            fs::write(dir.join(format!("strata-2026110{d}.log")), b"x").unwrap();
        }
        fs::write(dir.join("session-x.log"), b"x").unwrap();
        prune_daily(&dir, 2);
        let mut left: Vec<String> = fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().into_string().unwrap()).collect();
        left.sort();
        assert_eq!(left, ["session-x.log", "strata-20261104.log", "strata-20261105.log"]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn session_file_stops_at_cap() {
        let p = std::env::temp_dir().join(format!("strata_cap_{}.log", std::process::id()));
        let mut f = CappedFile::create(&p, 30);
        f.always("# header");
        for _ in 0..10 {
            f.line("0123456789");
        }
        f.always("# exit");
        drop(f);
        let text = fs::read_to_string(&p).unwrap();
        assert_eq!(text.matches("0123456789").count(), 2);
        assert!(text.contains("log stopped") && text.ends_with("# exit\n"));
        let _ = fs::remove_file(p);
    }

    #[test]
    fn strips_color_codes() {
        assert_eq!(strip_ansi("\u{1b}[36m[1][00:16:36.639] libkernel\u{1b}[0m done"), "[1][00:16:36.639] libkernel done");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn throttle_caps_lines() {
        let mut t = Throttle::new(3);
        assert!(t.allow().0 && t.allow().0 && t.allow().0);
        assert!(!t.allow().0);
        assert!(!t.allow().0);
        t.window -= std::time::Duration::from_secs(2);
        let (ok, skipped) = t.allow();
        assert!(ok);
        assert_eq!(skipped, Some(2));
    }

    #[test]
    fn classify_lines() {
        assert_eq!(classify("[Error] vk device lost"), "error");
        assert_eq!(classify("WARN: unimplemented syscall"), "warn");
        assert_eq!(classify("frame 120"), "info");
    }

    #[test]
    fn ring_and_filter() {
        log("info", "test", "hello");
        log("debug", "test", "hidden at info level");
        let all = since(0);
        assert!(all.iter().any(|l| l.msg == "hello"));
        assert!(!all.iter().any(|l| l.msg.contains("hidden")));
    }
}
