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

pub fn log(level: &'static str, source: &str, msg: &str) {
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
