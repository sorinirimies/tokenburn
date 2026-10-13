//! Small number-formatting helpers shared by the front-ends.

/// `1234567` → `"1,234,567"`.
pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// `1_234_567` → `"1.23M"`, `12_345` → `"12.3k"`, small numbers unchanged.
pub fn compact(n: u64) -> String {
    let f = n as f64;
    if n >= 1_000_000_000 {
        format!("{:.2}B", f / 1e9)
    } else if n >= 1_000_000 {
        format!("{:.2}M", f / 1e6)
    } else if n >= 10_000 {
        format!("{:.1}k", f / 1e3)
    } else {
        n.to_string()
    }
}

/// `1.5` → `"$1.50"`: cents, so small costs like `$0.04` stay readable
/// (the JSON output keeps the full precision).
pub fn usd(v: f64) -> String {
    format!("${v:.2}")
}

/// A share as a whole percentage: `0.486` of the total → `"49%"`.
pub fn percent(fraction: f64) -> String {
    format!("{:.0}%", fraction * 100.0)
}

/// Tokens for an axis: `20M`, `2.5M`, `15k`, `800` — no trailing zeros.
pub fn axis_tokens(v: f64) -> String {
    let (scaled, unit) = match v {
        v if v >= 1e9 => (v / 1e9, "B"),
        v if v >= 1e6 => (v / 1e6, "M"),
        v if v >= 1e3 => (v / 1e3, "k"),
        v => (v, ""),
    };
    let s = format!("{scaled:.1}");
    format!("{}{unit}", s.strip_suffix(".0").unwrap_or(&s))
}

/// A short x-axis label: drop the year (`2026-05-10` → `05-10`, `2026-05-10 13:00` → `13:00`).
pub fn short_label(label: &str) -> String {
    let tail = match label.len() {
        16 => label.get(11..),
        10 => label.get(5..),
        _ => None,
    };
    tail.unwrap_or(label).to_string()
}

/// `pi:Users-me-Projects-tokenburn` → `pi:tokenburn` (last path-ish segment).
pub fn short_name(full: &str) -> String {
    match full.split_once(':') {
        Some((tool, rest)) => {
            let last = rest
                .rsplit(['-', '/', '\\'])
                .find(|s| !s.is_empty())
                .unwrap_or(rest);
            format!("{tool}:{last}")
        }
        None => full.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_label_multibyte_is_safe() {
        assert_eq!(short_label("abcd\u{51b}xyz1"), "abcd\u{51b}xyz1");
        assert_eq!(short_label("2026-05-10"), "05-10");
    }

    #[test]
    fn thousands_groups() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn compact_scales() {
        assert_eq!(compact(999), "999");
        assert_eq!(compact(12_345), "12.3k");
        assert_eq!(compact(1_500_000), "1.50M");
        assert_eq!(compact(2_000_000_000), "2.00B");
    }

    #[test]
    fn usd_four_decimals() {
        assert_eq!(usd(1.5), "$1.50");
        assert_eq!(usd(0.0), "$0.00");
        assert_eq!(usd(0.04), "$0.04", "small costs stay readable");
        assert_eq!(usd(12.34), "$12.34");
        assert_eq!(usd(165.9051), "$165.91");
        assert_eq!(usd(1234.56), "$1234.56");
    }

    #[test]
    fn percentages_are_whole_numbers() {
        assert_eq!(percent(0.486), "49%");
        assert_eq!(percent(1.0), "100%");
        assert_eq!(percent(0.0), "0%");
        assert_eq!(percent(0.004), "0%");
    }

    #[test]
    fn axis_tokens_have_no_trailing_zeros() {
        assert_eq!(axis_tokens(20_000_000.0), "20M");
        assert_eq!(axis_tokens(2_500_000.0), "2.5M");
        assert_eq!(axis_tokens(15_000.0), "15k");
        assert_eq!(axis_tokens(800.0), "800");
        assert_eq!(axis_tokens(0.0), "0");
        assert_eq!(axis_tokens(3_000_000_000.0), "3B");
    }

    #[test]
    fn x_labels_drop_the_year() {
        assert_eq!(short_label("2026-05-10"), "05-10");
        assert_eq!(short_label("2026-05-10 13:00"), "13:00");
        assert_eq!(short_label("2026-05"), "2026-05");
    }

    #[test]
    fn short_names_keep_the_tool_and_the_last_path_segment() {
        assert_eq!(short_name("pi:Users-me-Projects-tokenburn"), "pi:tokenburn");
        assert_eq!(short_name("zed:claude-sonnet"), "zed:sonnet");
        assert_eq!(short_name("plain"), "plain");
    }
}
