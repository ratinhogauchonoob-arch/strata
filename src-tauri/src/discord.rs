//! Discord Rich Presence: "Playing <game> · Strata" on the user's Discord profile (opt-in).
//! Talks to the Discord app on this PC over its local pipe; nothing goes through Strata's servers.

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The "Discord is not open" note is a warning once, then quiet: most players simply don't run it.
static WARNED: AtomicBool = AtomicBool::new(false);

/// Application ID from discord.com/developers (public; safe to ship). Empty = feature off.
pub const APP_ID: &str = "1574113742936084731";

static CLIENT: Mutex<Option<DiscordIpcClient>> = Mutex::new(None);

pub fn available() -> bool {
    !APP_ID.is_empty()
}

/// Text shown under the game name.
pub fn details_line(engine_name: &str, platform: &str) -> String {
    format!("{} · {engine_name}", platform.to_uppercase())
}

/// Shows the game on a background thread (Discord may be closed: then nothing happens).
pub fn playing(title: String, engine_name: String, platform: String, started_unix: i64) {
    let details = details_line(&engine_name, &platform);
    show(title, details, started_unix);
}

/// In the menus: "Browsing the library · 12 games".
pub fn idle(games: usize, since_unix: i64) {
    let state = match games {
        0 => "Setting up".to_string(),
        1 => "1 game in the library".to_string(),
        n => format!("{n} games in the library"),
    };
    show("Browsing the library".into(), state, since_unix);
}

fn show(title: String, details: String, started_unix: i64) {
    if !available() {
        return;
    }
    std::thread::spawn(move || {
        let mut slot = CLIENT.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            let mut c = DiscordIpcClient::new(APP_ID);
            if let Err(e) = c.connect() {
                let msg = format!("Discord app not reachable (is the Discord desktop app open?): {e}");
                if WARNED.swap(true, Ordering::Relaxed) {
                    crate::logs::debug("discord", &msg);
                } else {
                    crate::logs::warn("discord", &msg);
                }
                return;
            }
            WARNED.store(false, Ordering::Relaxed);
            crate::logs::info("discord", "Connected to Discord");
            *slot = Some(c);
        }
        let act = activity::Activity::new()
            .details(title.as_str())
            .state(details.as_str())
            .timestamps(activity::Timestamps::new().start(started_unix))
            .assets(activity::Assets::new().large_image("strata").large_text("Strata launcher"));
        if let Some(c) = slot.as_mut() {
            match c.set_activity(act) {
                Ok(()) => crate::logs::info("discord", &format!("Showing \"{title}\" on Discord")),
                Err(e) => {
                    crate::logs::warn("discord", &format!("Discord did not accept the activity: {e}"));
                    *slot = None; // Discord was closed; reconnect next time
                }
            }
        }
    });
}

pub fn clear() {
    if !available() {
        return;
    }
    std::thread::spawn(|| {
        let mut slot = CLIENT.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = slot.as_mut() {
            let _ = c.clear_activity();
            let _ = c.close();
        }
        *slot = None;
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn line() {
        assert_eq!(super::details_line("KytyPS5", "ps5"), "PS5 · KytyPS5");
    }
}
