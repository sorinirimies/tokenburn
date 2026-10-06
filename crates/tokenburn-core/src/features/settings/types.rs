use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Persisted user settings. Unknown fields are ignored and missing ones get
/// their default, so the file stays forward/backward compatible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Theme name (see [`crate::THEME_NAMES`]).
    pub theme: String,
    /// Display currency code (`"USD"`, `"EUR"`, …; see [`crate::CURRENCIES`]).
    pub currency: String,
    /// Your own exchange rates, code → units per US dollar, overriding the built-in
    /// approximate ones: `{"EUR": 0.91}`. Never written by TokenBurn itself.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub rates: BTreeMap<String, f64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: crate::THEME_NAMES[0].to_string(),
            currency: "USD".to_string(),
            rates: BTreeMap::new(),
        }
    }
}
