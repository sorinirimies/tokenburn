//! "About" information shared by every front-end: version, author, links, paths.
//!
//! The TUI, GUI and web UI all render [`about_rows`], so they can never drift apart.

/// Product name.
pub const NAME: &str = "TokenBurn";

/// Release version (the workspace version, e.g. `0.2.1`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One-line description.
pub const TAGLINE: &str = "AI token usage & cost from your local agent logs";

/// Developer name.
pub const AUTHOR: &str = "Sorin Irimies";

/// GitHub profile URL.
pub const GITHUB_PROFILE: &str = "https://github.com/sorinirimies";

/// Source repository.
pub const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

/// License identifier (SPDX).
pub const LICENSE: &str = env!("CARGO_PKG_LICENSE");

/// `v0.2.1` — what the UIs show next to the product name.
pub fn version_label() -> String {
    format!("v{VERSION}")
}

/// `TokenBurn v0.2.1`.
pub fn title() -> String {
    format!("{NAME} {}", version_label())
}

/// Bug tracker.
pub fn issues_url() -> String {
    format!("{REPO_URL}/issues")
}

/// `macos aarch64`, `linux x86_64`, …
pub fn platform() -> String {
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Where the shared settings file lives.
pub fn settings_file() -> String {
    crate::settings::settings_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unknown)".into())
}

/// Whether an About value is a URL.
pub fn is_link(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

/// `(label, value)` rows of the About tab.
pub fn about_rows() -> Vec<(&'static str, String)> {
    vec![
        ("Version", VERSION.to_string()),
        ("Developer", AUTHOR.to_string()),
        ("GitHub", GITHUB_PROFILE.to_string()),
        ("Repository", REPO_URL.to_string()),
        ("Issues", issues_url()),
        ("License", LICENSE.to_string()),
        ("Platform", platform()),
        ("Settings file", settings_file()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_carries_the_version() {
        assert_eq!(title(), format!("TokenBurn v{VERSION}"));
        assert!(version_label().starts_with('v'));
    }

    #[test]
    fn links_are_detected() {
        assert!(is_link(GITHUB_PROFILE));
        assert!(!is_link("MIT"));
        assert!(about_rows().iter().any(|(k, _)| *k == "Version"));
    }
}
