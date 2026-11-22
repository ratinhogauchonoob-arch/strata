//! Discord Rich Presence: "Playing <game> · Strata" on the user's Discord profile (opt-in).
//! Talks to the Discord app on this PC over its local pipe; nothing goes through Strata's servers.

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::Mutex;

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
    if !available() {
        return;
    }
    std::thread::spawn(move || {
        let mut slot = CLIENT.lock().unwrap();
        if slot.is_none() {
            let mut c = DiscordIpcClient::new(APP_ID);
            if c.connect().is_err() {
                return; // Discord not running
            }
            *slot = Some(c);
        }
        let details = details_line(&engine_name, &platform);
        let act = activity::Activity::new()
            .details(title.as_str())
            .state(details.as_str())
            .timestamps(activity::Timestamps::new().start(started_unix))
            .assets(activity::Assets::new().large_image("strata").large_text("Strata launcher"));
        if let Some(c) = slot.as_mut() {
            if c.set_activity(act).is_err() {
                *slot = None; // Discord was closed; reconnect next time
            }
        }
    });
}

pub fn clear() {
    if !available() {
        return;
    }
    std::thread::spawn(|| {
        let mut slot = CLIENT.lock().unwrap();
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
