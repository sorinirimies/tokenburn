//! "Follow system" appearance: detect the OS dark/light setting and map it to a preset.

/// Persisted `Settings::theme` meaning "follow the operating system's light/dark setting".
pub const SYSTEM_THEME_NAME: &str = "System";

/// Preset used while the system is in dark mode (index into `THEME_NAMES`).
pub const SYSTEM_DARK_INDEX: usize = 0; // "Default"
/// Preset used while the system is in light mode (index into `THEME_NAMES`).
pub const SYSTEM_LIGHT_INDEX: usize = 1; // "Default Light"

/// Whether `name` is the "follow system" pseudo-theme (case-insensitive).
pub fn is_system_name(name: &str) -> bool {
    name.eq_ignore_ascii_case(SYSTEM_THEME_NAME)
}

/// The preset index to show for a given system appearance.
pub fn system_theme_index(dark: bool) -> usize {
    if dark {
        SYSTEM_DARK_INDEX
    } else {
        SYSTEM_LIGHT_INDEX
    }
}

/// Parse `defaults read -g AppleInterfaceStyle` output (macOS): prints `Dark` in dark mode.
pub fn parse_macos(out: &str) -> bool {
    out.trim().eq_ignore_ascii_case("dark")
}

/// Parse `gsettings get org.gnome.desktop.interface color-scheme` (`'prefer-dark'`, …).
pub fn parse_gsettings(out: &str) -> Option<bool> {
    let v = out.trim().trim_matches('\'');
    match v {
        "prefer-dark" => Some(true),
        "prefer-light" | "default" => Some(false),
        _ => None,
    }
}

/// Parse `reg query … /v AppsUseLightTheme` output (Windows): `0x0` means dark.
pub fn parse_windows(out: &str) -> Option<bool> {
    let v = out.split_whitespace().last()?;
    match v {
        "0x0" => Some(true),
        "0x1" => Some(false),
        _ => None,
    }
}

#[allow(dead_code)]
fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let o = std::process::Command::new(cmd).args(args).output().ok()?;
    Some(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Is the OS in dark mode? `None` when it can't be determined (callers pick a default).
pub fn detect_system_dark() -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        // Key is absent in light mode (command fails, empty stdout).
        Some(parse_macos(
            &run("defaults", &["read", "-g", "AppleInterfaceStyle"]).unwrap_or_default(),
        ))
    }
    #[cfg(target_os = "windows")]
    {
        parse_windows(&run(
            "reg",
            &[
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ],
        )?)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Some(d) = run(
            "gsettings",
            &["get", "org.gnome.desktop.interface", "color-scheme"],
        )
        .and_then(|o| parse_gsettings(&o))
        {
            return Some(d);
        }
        std::env::var("GTK_THEME")
            .ok()
            .map(|t| t.to_ascii_lowercase().contains("dark"))
    }
}

/// Preset index matching the current system appearance (dark if undetectable).
pub fn current_system_theme_index() -> usize {
    system_theme_index(detect_system_dark().unwrap_or(true))
}

/// A theme as the user names it: a preset, or "follow the system".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    System,
    /// Index into [`super::THEME_NAMES`].
    Preset(usize),
}

impl ThemeChoice {
    /// Parse `"System"` or a preset name (case-insensitive); `None` when unknown.
    pub fn parse(name: &str) -> Option<Self> {
        if is_system_name(name.trim()) {
            Some(Self::System)
        } else {
            super::find_theme(name).map(Self::Preset)
        }
    }

    /// The preset to paint right now (System resolves via the OS setting).
    pub fn resolve(self) -> usize {
        match self {
            Self::System => current_system_theme_index(),
            Self::Preset(i) => i,
        }
    }

    /// The name that is persisted in settings.
    pub fn name(self) -> &'static str {
        match self {
            Self::System => SYSTEM_THEME_NAME,
            Self::Preset(i) => super::THEME_NAMES
                .get(i)
                .copied()
                .unwrap_or(super::THEME_NAMES[0]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_name_is_case_insensitive_and_not_a_preset() {
        assert!(is_system_name("System") && is_system_name("system"));
        assert!(!is_system_name("Nord"));
        assert!(!super::super::THEME_NAMES.iter().any(|n| is_system_name(n)));
    }

    #[test]
    fn dark_and_light_map_to_matching_presets() {
        assert!(super::super::theme_by_index(system_theme_index(true)).is_dark);
        assert!(!super::super::theme_by_index(system_theme_index(false)).is_dark);
    }

    #[test]
    fn theme_choice_parses_names_and_resolves() {
        assert_eq!(ThemeChoice::parse(" system "), Some(ThemeChoice::System));
        assert_eq!(
            ThemeChoice::parse("nord"),
            super::super::find_theme("Nord").map(ThemeChoice::Preset)
        );
        assert_eq!(ThemeChoice::parse("nope"), None);
        assert_eq!(ThemeChoice::System.name(), "System");
        assert_eq!(ThemeChoice::Preset(1).name(), "Default Light");
        assert_eq!(ThemeChoice::Preset(9999).name(), "Default");
        assert_eq!(ThemeChoice::Preset(5).resolve(), 5);
        assert!(ThemeChoice::System.resolve() < super::super::THEME_COUNT);
    }

    #[test]
    fn parsers() {
        assert!(parse_macos("Dark\n"));
        assert!(!parse_macos(""));
        assert_eq!(parse_gsettings("'prefer-dark'\n"), Some(true));
        assert_eq!(parse_gsettings("'default'"), Some(false));
        assert_eq!(parse_gsettings("garbage"), None);
        assert_eq!(
            parse_windows("    AppsUseLightTheme    REG_DWORD    0x0\r\n"),
            Some(true)
        );
        assert_eq!(
            parse_windows("AppsUseLightTheme REG_DWORD 0x1"),
            Some(false)
        );
        assert_eq!(parse_windows(""), None);
    }
}
