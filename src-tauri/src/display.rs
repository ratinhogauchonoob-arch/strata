//! Refresh rate of the monitor Strata's window is on, for the "Auto" virtual refresh rate.

/// Refresh rate (Hz) of a display by its system name (Windows: "\\.\DISPLAY1").
#[cfg(windows)]
pub fn refresh_rate(display_name: &str) -> Option<u32> {
    #[link(name = "user32")]
    extern "system" {
        fn EnumDisplaySettingsW(device: *const u16, mode: u32, devmode: *mut u32) -> i32;
    }
    const ENUM_CURRENT_SETTINGS: u32 = 0xFFFF_FFFF;
    // DEVMODEW is 220 bytes; dmSize sits at byte 68 and dmDisplayFrequency at byte 184.
    let mut dm = [0u32; 55];
    dm[17] = 220; // dmSize (low u16) + dmDriverExtra 0 (high u16)
    let name: Vec<u16> = display_name.encode_utf16().chain(std::iter::once(0)).collect();
    let ok = unsafe { EnumDisplaySettingsW(name.as_ptr(), ENUM_CURRENT_SETTINGS, dm.as_mut_ptr()) };
    let hz = dm[46];
    (ok != 0 && hz > 1).then_some(hz)
}

#[cfg(not(windows))]
pub fn refresh_rate(_display_name: &str) -> Option<u32> {
    None
}

/// The value passed to the engine: a fixed setting, or the monitor's rate for Auto (0).
pub fn resolve_vblank(setting: u32, detected: Option<u32>) -> u32 {
    if setting != 0 {
        return setting.clamp(24, 500);
    }
    detected.filter(|hz| (24..=500).contains(hz)).unwrap_or(60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_follows_monitor() {
        assert_eq!(resolve_vblank(0, Some(280)), 280);
        assert_eq!(resolve_vblank(0, Some(144)), 144);
        assert_eq!(resolve_vblank(0, None), 60);
        assert_eq!(resolve_vblank(0, Some(1)), 60, "nonsense from the driver");
        assert_eq!(resolve_vblank(75, Some(280)), 75, "a fixed value wins");
    }
}
