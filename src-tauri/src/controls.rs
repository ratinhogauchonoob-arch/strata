//! Keyboard and mouse controls. Strata keeps one binding table and writes it in the format
//! each engine reads. Engines with fixed controls (KytyPS5, SharpEmu) are left alone.

use crate::engines::{self, Kind};
use crate::settings::ControlSettings;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Actions in the order the UI lists them: (id, label, default key).
pub const ACTIONS: &[(&str, &str, &str)] = &[
    ("cross", "Cross", "space"),
    ("circle", "Circle", "c"),
    ("square", "Square", "f"),
    ("triangle", "Triangle", "r"),
    ("l1", "L1", "q"),
    ("r1", "R1", "e"),
    ("l2", "L2", "mouse_right"),
    ("r2", "R2", "mouse_left"),
    ("l3", "L3", "lshift"),
    ("r3", "R3", "v"),
    ("options", "Options", "enter"),
    ("touchpad", "Touchpad", "tab"),
    ("up", "D-pad up", "up"),
    ("down", "D-pad down", "down"),
    ("left", "D-pad left", "left"),
    ("right", "D-pad right", "right"),
    ("ls_up", "Left stick up", "w"),
    ("ls_down", "Left stick down", "s"),
    ("ls_left", "Left stick left", "a"),
    ("ls_right", "Left stick right", "d"),
    ("rs_up", "Right stick up", "i"),
    ("rs_down", "Right stick down", "k"),
    ("rs_left", "Right stick left", "j"),
    ("rs_right", "Right stick right", "l"),
];

/// Key names Strata understands, with their shadPS4 and SDL (AnyPS5) spellings.
/// (strata, shadps4, anyps5)
pub const KEYS: &[(&str, &str, &str)] = &[
    ("space", "space", "KEY:Space"),
    ("enter", "enter", "KEY:Return"),
    ("escape", "escape", "KEY:Escape"),
    ("tab", "tab", "KEY:Tab"),
    ("backspace", "backspace", "KEY:Backspace"),
    ("lshift", "lshift", "KEY:Left Shift"),
    ("rshift", "rshift", "KEY:Right Shift"),
    ("lctrl", "lctrl", "KEY:Left Ctrl"),
    ("rctrl", "rctrl", "KEY:Right Ctrl"),
    ("lalt", "lalt", "KEY:Left Alt"),
    ("ralt", "ralt", "KEY:Right Alt"),
    ("up", "up", "KEY:Up"),
    ("down", "down", "KEY:Down"),
    ("left", "left", "KEY:Left"),
    ("right", "right", "KEY:Right"),
    ("mouse_left", "leftbutton", "MOUSE:Left"),
    ("mouse_right", "rightbutton", "MOUSE:Right"),
    ("mouse_middle", "middlebutton", "MOUSE:Middle"),
];

fn simple_key(k: &str) -> bool {
    let b = k.as_bytes();
    (b.len() == 1 && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit()))
        || (k.starts_with('f') && k[1..].parse::<u32>().is_ok_and(|n| (1..=12).contains(&n)))
        || (k.starts_with("kp") && k.len() == 3 && b[2].is_ascii_digit())
}

pub fn is_key(k: &str) -> bool {
    simple_key(k) || KEYS.iter().any(|x| x.0 == k)
}

pub fn is_action(a: &str) -> bool {
    ACTIONS.iter().any(|x| x.0 == a)
}

pub fn default_keys() -> BTreeMap<String, String> {
    ACTIONS.iter().map(|(a, _, k)| (a.to_string(), k.to_string())).collect()
}

fn shad_key(k: &str) -> Option<String> {
    if simple_key(k) {
        return Some(k.to_string());
    }
    KEYS.iter().find(|x| x.0 == k).map(|x| x.1.to_string())
}

fn sdl_key(k: &str) -> Option<String> {
    if let Some(x) = KEYS.iter().find(|x| x.0 == k) {
        return Some(x.2.to_string());
    }
    if !simple_key(k) {
        return None;
    }
    if let Some(n) = k.strip_prefix("kp") {
        return Some(format!("KEY:Keypad {n}"));
    }
    Some(format!("KEY:{}", k.to_ascii_uppercase()))
}

fn key_of<'a>(c: &'a ControlSettings, action: &str) -> Option<&'a str> {
    c.keys.get(action).map(String::as_str).filter(|k| is_key(k))
}

/// shadPS4 input_config file (one per game, "<TITLE_ID>.ini").
pub fn shadps4_ini(c: &ControlSettings) -> String {
    fn name(a: &str) -> &str {
        match a {
        "touchpad" => "touchpad_center",
        "up" => "pad_up",
        "down" => "pad_down",
        "left" => "pad_left",
        "right" => "pad_right",
        "ls_left" => "axis_left_x_minus",
        "ls_right" => "axis_left_x_plus",
        "ls_up" => "axis_left_y_minus",
        "ls_down" => "axis_left_y_plus",
        "rs_left" => "axis_right_x_minus",
        "rs_right" => "axis_right_x_plus",
        "rs_up" => "axis_right_y_minus",
        "rs_down" => "axis_right_y_plus",
        other => other,
        }
    }
    let mut out = String::from("# Written by Strata. Change controls in Strata → Settings → Controller.\n\n# Keyboard and mouse\n");
    for (a, _, _) in ACTIONS {
        if let Some(k) = key_of(c, a).and_then(shad_key) {
            out.push_str(&format!("{} = {}\n", name(a), k));
        }
    }
    out.push_str("\n# Controller\n");
    let (cross, circle) = if c.swap_cross_circle { ("circle", "cross") } else { ("cross", "circle") };
    out.push_str(&format!("cross = {cross}\ncircle = {circle}\n"));
    for b in ["triangle", "square", "l1", "l2", "l3", "r1", "r2", "r3", "options", "pad_up", "pad_down", "pad_left", "pad_right"] {
        out.push_str(&format!("{b} = {b}\n"));
    }
    out.push_str("touchpad_center = back\n");
    for ax in ["axis_left_x", "axis_left_y", "axis_right_x", "axis_right_y"] {
        out.push_str(&format!("{ax} = {ax}\n"));
    }
    let dz = (c.deadzone.min(90) * 127 / 100).max(1);
    out.push_str(&format!("\nanalog_deadzone = leftjoystick, {dz}, 127\nanalog_deadzone = rightjoystick, {dz}, 127\n"));
    out
}

/// AnyPS5 anyps5-input.ini.
pub fn anyps5_ini(c: &ControlSettings) -> String {
    let name = |a: &str| -> String {
        match a {
            "touchpad" => "TouchLeft".into(),
            "ls_up" => "LeftStickUp".into(),
            "ls_down" => "LeftStickDown".into(),
            "ls_left" => "LeftStickLeft".into(),
            "ls_right" => "LeftStickRight".into(),
            "rs_up" => "RightStickUp".into(),
            "rs_down" => "RightStickDown".into(),
            "rs_left" => "RightStickLeft".into(),
            "rs_right" => "RightStickRight".into(),
            "l1" | "r1" | "l2" | "r2" | "l3" | "r3" => a.to_ascii_uppercase(),
            other => {
                let mut s = other.to_string();
                s[..1].make_ascii_uppercase();
                s
            }
        }
    };
    let mut out = String::from("# Written by Strata. Change controls in Strata → Settings → Controller.\n");
    for (a, _, _) in ACTIONS {
        if let Some(k) = key_of(c, a).and_then(sdl_key) {
            out.push_str(&format!("{} = {}\n", name(a), k));
        }
    }
    if c.swap_cross_circle {
        out.push_str("SDLButton:a = Circle\nSDLButton:b = Cross\n");
    }
    let dz = c.deadzone.min(90);
    out.push_str(&format!("LeftStickDeadzone = {dz}\nRightStickDeadzone = {dz}\n"));
    out
}

/// Where Strata keeps the AnyPS5 controls for a game (passed through ANYPS5_INPUT_CONFIG).
pub fn anyps5_file(data: &Path, key: &str) -> PathBuf {
    data.join("controls").join("anyps5").join(format!("{key}.ini"))
}

/// Writes the controls for one launch. Returns the file to hand to the engine, if any.
/// A file the user wrote by hand is kept once as `.bak` before Strata replaces it.
pub fn write_for(kind: Kind, c: &ControlSettings, data: &Path, title_id: &str, game_key: &str) -> Result<Option<PathBuf>, String> {
    if !c.enabled {
        return Ok(None);
    }
    let (path, text) = match kind {
        Kind::ShadPs4 => {
            let id = if title_id.is_empty() { "default" } else { title_id };
            (engines::root_dir(data, "shadps4").join("user").join("input_config").join(format!("{id}.ini")), shadps4_ini(c))
        }
        Kind::AnyPs5 => (anyps5_file(data, game_key), anyps5_ini(c)),
        Kind::Kyty | Kind::SharpEmu => return Ok(None),
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    if let Ok(old) = std::fs::read_to_string(&path) {
        let bak = path.with_extension("ini.bak");
        if !old.starts_with("# Written by Strata") && !bak.exists() {
            let _ = std::fs::write(&bak, old);
        }
    }
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ControlSettings {
        ControlSettings { enabled: true, ..Default::default() }
    }

    #[test]
    fn keys_are_validated() {
        for k in ["a", "z", "0", "f1", "f12", "kp5", "space", "mouse_left"] {
            assert!(is_key(k), "{k}");
        }
        for k in ["", "f13", "aa", "kp", "Space", "mouse"] {
            assert!(!is_key(k), "{k}");
        }
        assert!(default_keys().values().all(|k| is_key(k)));
        assert_eq!(default_keys().len(), ACTIONS.len());
    }

    #[test]
    fn shadps4_format() {
        let mut c = cfg();
        c.swap_cross_circle = true;
        c.deadzone = 50;
        let t = shadps4_ini(&c);
        assert!(t.contains("cross = space\n"));
        assert!(t.contains("r2 = leftbutton\n"));
        assert!(t.contains("axis_left_x_minus = a\n"));
        assert!(t.contains("touchpad_center = tab\n"));
        assert!(t.contains("cross = circle\ncircle = cross\n"));
        assert!(t.contains("analog_deadzone = leftjoystick, 63, 127"));
    }

    #[test]
    fn anyps5_format() {
        let mut c = cfg();
        c.keys.insert("cross".into(), "kp2".into());
        c.swap_cross_circle = true;
        let t = anyps5_ini(&c);
        assert!(t.contains("Cross = KEY:Keypad 2\n"));
        assert!(t.contains("Circle = KEY:C\n"));
        assert!(t.contains("R2 = MOUSE:Left\n"));
        assert!(t.contains("LeftStickLeft = KEY:A\n"));
        assert!(t.contains("Options = KEY:Return\n"));
        assert!(t.contains("SDLButton:a = Circle"));
        assert!(t.contains("LeftStickDeadzone = 8"));
    }

    #[test]
    fn writes_and_keeps_user_file() {
        let data = std::env::temp_dir().join(format!("strata_ctl_{}", std::process::id()));
        let file = engines::root_dir(&data, "shadps4").join("user/input_config/CUSA1.ini");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "cross = n\n").unwrap();
        let p = write_for(Kind::ShadPs4, &cfg(), &data, "CUSA1", "k").unwrap().unwrap();
        assert_eq!(p, file);
        assert!(std::fs::read_to_string(&file).unwrap().starts_with("# Written by Strata"));
        assert_eq!(std::fs::read_to_string(file.with_extension("ini.bak")).unwrap(), "cross = n\n");
        // Disabled or fixed-control engines write nothing.
        assert!(write_for(Kind::Kyty, &cfg(), &data, "X", "k").unwrap().is_none());
        assert!(write_for(Kind::ShadPs4, &ControlSettings::default(), &data, "X", "k").unwrap().is_none());
        let _ = std::fs::remove_dir_all(data);
    }
}
