//! Persisted user settings (currently just the tray-icon theme) plus detection
//! of the current Windows taskbar theme for `Auto` mode.
//!
//! The choice is stored under `HKCU\Software\razer-battery-report\Theme`.

use log::warn;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

use crate::icon::Theme;

const APP_KEY: &str = r"Software\razer-battery-report";
const THEME_VALUE: &str = "Theme";
const PERSONALIZE_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

/// The user's tray-icon theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeSetting {
    /// Follow the Windows taskbar theme.
    Auto,
    /// Force the light-taskbar palette (dark digits).
    Light,
    /// Force the dark-taskbar palette (bright digits).
    Dark,
}

impl ThemeSetting {
    fn as_str(self) -> &'static str {
        match self {
            ThemeSetting::Auto => "auto",
            ThemeSetting::Light => "light",
            ThemeSetting::Dark => "dark",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "light" => ThemeSetting::Light,
            "dark" => ThemeSetting::Dark,
            _ => ThemeSetting::Auto,
        }
    }

    /// Resolve to a concrete icon palette, detecting the Windows taskbar theme
    /// when set to `Auto`.
    pub fn resolve(self) -> Theme {
        match self {
            ThemeSetting::Light => Theme::Light,
            ThemeSetting::Dark => Theme::Dark,
            ThemeSetting::Auto => detect_system_theme(),
        }
    }
}

/// Read the persisted theme setting (defaults to `Auto`).
pub fn load() -> ThemeSetting {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(APP_KEY)
        .and_then(|k| k.get_value::<String, _>(THEME_VALUE))
        .map(|s| ThemeSetting::parse(&s))
        .unwrap_or(ThemeSetting::Auto)
}

/// Persist the theme setting under `HKCU`.
pub fn save(setting: ThemeSetting) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.create_subkey(APP_KEY) {
        Ok((key, _)) => {
            let value = setting.as_str().to_string();
            if let Err(e) = key.set_value(THEME_VALUE, &value) {
                warn!("Failed to save theme setting: {}", e);
            }
        }
        Err(e) => warn!("Failed to open settings registry key: {}", e),
    }
}

/// Detect whether the Windows taskbar uses a light or dark theme.
///
/// `SystemUsesLightTheme == 1` means the taskbar/system tray is light, so we
/// use the dark-digit palette; otherwise the bright-digit palette.
fn detect_system_theme() -> Theme {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let light = hkcu
        .open_subkey_with_flags(PERSONALIZE_KEY, KEY_READ)
        .and_then(|k| k.get_value::<u32, _>("SystemUsesLightTheme"))
        .map(|v| v == 1)
        .unwrap_or(false);

    if light {
        Theme::Light
    } else {
        Theme::Dark
    }
}
