use serde::{Deserialize, Serialize};

/// Persisted user settings. Unknown fields are ignored and missing ones get
/// their default, so the file stays forward/backward compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Theme name (see [`crate::THEME_NAMES`]).
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: crate::THEME_NAMES[0].to_string(),
        }
    }
}
