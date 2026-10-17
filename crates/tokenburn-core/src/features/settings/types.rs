use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Persisted user settings. Unknown fields are ignored and missing ones get
/// their default, so the file stays forward/backward compatible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Theme name (see [`crate::THEME_NAMES`]) or `"System"` to follow the OS light/dark setting.
    pub theme: String,
    /// Display currency code (`"USD"`, `"EUR"`, …; see [`crate::CURRENCIES`]).
    pub currency: String,
    /// Your own exchange rates, code → units per US dollar, overriding the built-in
    /// approximate ones: `{"EUR": 0.91}`. Never written by TokenBurn itself.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub rates: BTreeMap<String, f64>,
    /// What one million input tokens cost you, in US dollars. Prices the tokens
    /// saved by prompt caching, rtk and pi compaction
    /// (default [`crate::DEFAULT_INPUT_PRICE_PER_MTOK`]).
    pub input_price_per_mtok: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: crate::THEME_NAMES[0].to_string(),
            currency: "USD".to_string(),
            rates: BTreeMap::new(),
            input_price_per_mtok: crate::DEFAULT_INPUT_PRICE_PER_MTOK,
        }
    }
}

impl Settings {
    /// The input price to value savings at: the saved one when it is a sane
    /// positive number, otherwise the default.
    pub fn input_price(&self) -> f64 {
        let p = self.input_price_per_mtok;
        if p.is_finite() && p > 0.0 {
            p
        } else {
            crate::DEFAULT_INPUT_PRICE_PER_MTOK
        }
    }

    /// The saved theme as a [`crate::ThemeChoice`] (unknown names fall back to preset 0).
    pub fn theme_choice(&self) -> crate::ThemeChoice {
        crate::ThemeChoice::parse(&self.theme).unwrap_or(crate::ThemeChoice::Preset(0))
    }

    /// Whether the theme follows the OS light/dark setting.
    pub fn follow_system_theme(&self) -> bool {
        crate::is_system_name(self.theme.trim())
    }

    /// The preset index to paint now (`System` resolves via the OS; unknown names give 0).
    pub fn theme_index(&self) -> usize {
        self.theme_choice().resolve()
    }
}
