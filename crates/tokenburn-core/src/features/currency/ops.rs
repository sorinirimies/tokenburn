use std::collections::BTreeMap;

use super::types::{Currency, CURRENCIES};

/// The built-in currency with this code (case-insensitive).
pub fn find(code: &str) -> Option<Currency> {
    let code = code.trim();
    CURRENCIES
        .iter()
        .copied()
        .find(|c| c.code.eq_ignore_ascii_case(code))
}

/// Index of `code` in [`CURRENCIES`] (0 = USD when unknown).
pub fn index_of(code: &str) -> usize {
    CURRENCIES
        .iter()
        .position(|c| c.code.eq_ignore_ascii_case(code.trim()))
        .unwrap_or(0)
}

/// The currency for `code`, with the user's own rate from `rates` (code → units per
/// US dollar) when there is a sane one. An unknown code falls back to USD, a
/// non-positive or non-finite rate is ignored.
pub fn resolve(code: &str, rates: &BTreeMap<String, f64>) -> Currency {
    let mut cur = find(code).unwrap_or(Currency::USD);
    let own = rates
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(cur.code))
        .map(|(_, v)| *v)
        .filter(|v| v.is_finite() && *v > 0.0);
    if let Some(rate) = own {
        cur.per_usd = rate;
        cur.custom_rate = true;
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sane() {
        assert_eq!(CURRENCIES[0].code, "USD");
        assert_eq!(CURRENCIES[0].per_usd, 1.0, "USD is the unit the logs use");
        let mut codes: Vec<&str> = CURRENCIES.iter().map(|c| c.code).collect();
        let n = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), n, "codes are unique");
        assert!(CURRENCIES
            .iter()
            .all(|c| c.per_usd > 0.0 && c.code.len() == 3 && !c.symbol.is_empty()));
        // After USD the list is alphabetical, which is how pick lists show it.
        let rest: Vec<&str> = CURRENCIES[1..].iter().map(|c| c.code).collect();
        let mut sorted = rest.clone();
        sorted.sort_unstable();
        assert_eq!(rest, sorted);
    }

    #[test]
    fn the_requested_currencies_are_there() {
        let ron = find("RON").unwrap();
        assert_eq!(
            (ron.name, ron.format(10.0).as_str()),
            ("Romanian leu", "lei 46.00")
        );
        let rub = find("RUB").unwrap();
        assert_eq!((rub.name, rub.symbol), ("Russian ruble", "₽"));
        assert_eq!(rub.format(1.0), "₽90.00");
        // A spread across regions, and the zero-decimal ones really have none.
        for code in [
            "AED", "BGN", "HUF", "IDR", "ILS", "NGN", "PHP", "SAR", "THB", "TWD", "UAH", "VND",
        ] {
            assert!(find(code).is_some(), "{code}");
        }
        assert_eq!(find("HUF").unwrap().format(1.0), "Ft 360");
        assert_eq!(find("VND").unwrap().format(1.0), "₫24500");
        assert!(CURRENCIES.len() >= 40);
    }

    #[test]
    fn lookup_is_case_insensitive_and_unknown_codes_are_none() {
        assert_eq!(find("eur").unwrap().code, "EUR");
        assert_eq!(find(" JPY ").unwrap().code, "JPY");
        assert!(find("XYZ").is_none());
        assert_eq!(index_of("gbp"), index_of("GBP"));
        assert_eq!(index_of("nope"), 0);
    }

    #[test]
    fn formatting_uses_the_symbol_and_decimals() {
        assert_eq!(Currency::USD.format(1.5), "$1.50");
        assert_eq!(Currency::USD.format(0.04), "$0.04");
        let eur = find("EUR").unwrap();
        assert_eq!(eur.format(10.0), "€9.20");
        let jpy = find("JPY").unwrap();
        assert_eq!(jpy.format(1.4), "¥210", "no decimals for yen");
        let chf = find("CHF").unwrap();
        assert_eq!(chf.format(1.0), "CHF 0.88", "a code prefix gets a space");
    }

    #[test]
    fn axis_labels_drop_the_decimals_from_a_hundred_up() {
        let usd = Currency::USD;
        assert_eq!(usd.axis(250.0), "$250");
        assert_eq!(usd.axis(12.5), "$12.50");
        assert_eq!(usd.axis(0.25), "$0.25");
        assert_eq!(find("JPY").unwrap().axis(1500.0), "¥1500");
    }

    #[test]
    fn conversion_is_linear_and_round_trips() {
        let eur = find("EUR").unwrap();
        assert!((eur.convert(100.0) - 92.0).abs() < 1e-9);
        assert_eq!(eur.convert(0.0), 0.0);
        assert!((eur.convert(1.0) / eur.per_usd - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_user_rate_overrides_the_built_in_one() {
        let mut rates = BTreeMap::new();
        rates.insert("eur".to_string(), 0.5);
        let eur = resolve("EUR", &rates);
        assert_eq!(eur.per_usd, 0.5);
        assert!(eur.custom_rate && !eur.is_approximate());
        assert_eq!(eur.format(10.0), "€5.00");
        // Another currency is unaffected.
        assert!(resolve("GBP", &rates).is_approximate());
    }

    #[test]
    fn nonsense_rates_and_codes_are_ignored() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut rates = BTreeMap::new();
            rates.insert("EUR".to_string(), bad);
            let eur = resolve("EUR", &rates);
            assert_eq!(eur.per_usd, 0.92, "{bad} must not be used");
            assert!(!eur.custom_rate);
        }
        assert_eq!(resolve("XYZ", &BTreeMap::new()).code, "USD");
    }

    #[test]
    fn approximation_is_flagged_except_for_usd_and_user_rates() {
        assert_eq!(Currency::USD.approx_mark(), "");
        assert_eq!(find("EUR").unwrap().approx_mark(), " ≈");
    }

    #[test]
    fn parsing_and_display() {
        assert_eq!("chf".parse::<Currency>().unwrap().code, "CHF");
        let err = "foo".parse::<Currency>().unwrap_err();
        assert!(err.contains("unknown currency 'foo'") && err.contains("EUR"));
        assert_eq!(find("EUR").unwrap().to_string(), "EUR · Euro");
        assert_eq!(Currency::default(), Currency::USD);
    }
}
