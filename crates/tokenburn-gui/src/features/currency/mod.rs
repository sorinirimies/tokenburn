//! Display currency: the pick list entry. Costs are logged in US dollars; only the
//! display changes.

use std::fmt;

use tokenburn_core::CURRENCIES;

/// One entry of the currency pick list (compared by position, so a currency with
/// the user's own rate still matches its entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencyChoice {
    pub index: usize,
    pub code: &'static str,
    pub name: &'static str,
}

impl CurrencyChoice {
    /// The choice for `index` (clamped into range).
    pub fn at(index: usize) -> Self {
        let index = index.min(CURRENCIES.len() - 1);
        Self {
            index,
            code: CURRENCIES[index].code,
            name: CURRENCIES[index].name,
        }
    }

    /// The choice matching a currency code (USD when unknown).
    pub fn of(code: &str) -> Self {
        Self::at(tokenburn_core::features::currency::index_of(code))
    }

    /// Every currency, in the canonical order.
    pub fn all() -> Vec<CurrencyChoice> {
        (0..CURRENCIES.len()).map(Self::at).collect()
    }
}

impl fmt::Display for CurrencyChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} · {}", self.code, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_cover_every_currency_in_order() {
        let all = CurrencyChoice::all();
        assert_eq!(all.len(), CURRENCIES.len());
        assert_eq!(all[0].code, "USD");
        assert_eq!(all[0].to_string(), "USD · US dollar");
        assert_eq!(CurrencyChoice::of("eur").code, "EUR");
        assert_eq!(CurrencyChoice::of("nonsense").code, "USD");
        assert_eq!(CurrencyChoice::at(usize::MAX).code, "ZAR", "clamped");
    }
}
