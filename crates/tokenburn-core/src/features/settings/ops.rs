use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::types::Settings;
use crate::features::theme::{find_theme, THEME_NAMES};
use crate::utils::files::env_path;

/// `$TOKENBURN_CONFIG_DIR`, else `<config dir>/tokenburn`
/// (`~/.config/tokenburn`, `~/Library/Application Support/tokenburn`, `%APPDATA%\tokenburn`).
pub fn config_dir() -> Option<PathBuf> {
    env_path("TOKENBURN_CONFIG_DIR").or_else(|| dirs::config_dir().map(|d| d.join("tokenburn")))
}

/// Full path of `settings.json`.
pub fn settings_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("settings.json"))
}

/// Load settings; a missing or malformed file yields the defaults (the file is
/// left untouched so a hand edit can be recovered).
pub fn load() -> Settings {
    settings_path().map(|p| load_from(&p)).unwrap_or_default()
}

pub fn load_from(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Save settings atomically (write a temp file next to the target, then rename).
pub fn save(settings: &Settings) -> Result<()> {
    let path = settings_path().context("could not determine the config directory")?;
    save_to(&path, settings)
}

pub fn save_to(path: &Path, settings: &Settings) -> Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let json = serde_json::to_string_pretty(settings)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(json.as_bytes())?;
    tmp.persist(path)
        .map_err(|e| anyhow::anyhow!("writing {}: {e}", path.display()))?;
    Ok(())
}

/// Index of the saved theme in [`THEME_NAMES`] (0 when unset or unknown).
pub fn theme_index() -> usize {
    find_theme(&load().theme).unwrap_or(0)
}

/// Persist the theme at `index` (ignored when out of range).
pub fn save_theme(index: usize) -> Result<()> {
    let Some(name) = THEME_NAMES.get(index) else {
        return Ok(());
    };
    let mut s = load();
    if s.theme == *name {
        return Ok(());
    }
    s.theme = (*name).to_string();
    save(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(load_from(&d.path().join("nope.json")), Settings::default());
    }

    #[test]
    fn round_trips_atomically_and_creates_directories() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("deep/er/settings.json");
        save_to(
            &p,
            &Settings {
                theme: "Nord".into(),
            },
        )
        .unwrap();
        assert_eq!(load_from(&p).theme, "Nord");
        let leftovers: Vec<_> = std::fs::read_dir(p.parent().unwrap())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name() != "settings.json")
            .collect();
        assert!(leftovers.is_empty(), "no stray temp files");
    }

    #[test]
    fn malformed_json_falls_back_without_touching_the_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        std::fs::write(&p, "{ not json").unwrap();
        assert_eq!(load_from(&p), Settings::default());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "{ not json");
    }

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        std::fs::write(&p, r#"{"future_option": true}"#).unwrap();
        assert_eq!(load_from(&p), Settings::default());
        std::fs::write(&p, r#"{"theme":"Dracula","future_option":1}"#).unwrap();
        assert_eq!(load_from(&p).theme, "Dracula");
    }

    /// `TOKENBURN_CONFIG_DIR` is process-wide: one test owns it.
    #[test]
    fn the_env_override_drives_the_public_load_and_save_api() {
        let d = tempfile::tempdir().unwrap();
        // SAFETY: the only test in this binary that touches TOKENBURN_CONFIG_DIR.
        unsafe { std::env::set_var("TOKENBURN_CONFIG_DIR", d.path()) };

        assert_eq!(config_dir().unwrap(), d.path());
        assert_eq!(settings_path().unwrap(), d.path().join("settings.json"));
        assert_eq!(load(), Settings::default());
        assert_eq!(theme_index(), 0);

        save_theme(5).unwrap();
        assert_eq!(theme_index(), 5);
        assert_eq!(load().theme, THEME_NAMES[5]);

        // Same theme again, and an out-of-range index, are no-ops.
        let before = std::fs::metadata(settings_path().unwrap())
            .unwrap()
            .modified()
            .unwrap();
        save_theme(5).unwrap();
        save_theme(THEME_NAMES.len() + 10).unwrap();
        assert_eq!(theme_index(), 5);
        assert_eq!(
            std::fs::metadata(settings_path().unwrap())
                .unwrap()
                .modified()
                .unwrap(),
            before,
            "unchanged theme must not rewrite the file"
        );

        save(&Settings {
            theme: "Nord".into(),
        })
        .unwrap();
        assert_eq!(load().theme, "Nord");
        unsafe { std::env::remove_var("TOKENBURN_CONFIG_DIR") };
    }
}
