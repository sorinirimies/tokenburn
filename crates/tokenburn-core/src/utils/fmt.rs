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

/// `1.5` → `"$1.5"`: one decimal is plenty for what these numbers are used for
/// (the JSON output keeps the full precision).
pub fn usd(v: f64) -> String {
    format!("${v:.1}")
}

/// A cost on a chart axis or donut: `$250`, `$12.5`, and two decimals below a dollar
/// (ticks like `$0.25` would all collapse to `$0.3` with one).
pub fn usd_axis(v: f64) -> String {
    if v >= 100.0 {
        format!("${v:.0}")
    } else if v >= 1.0 {
        usd(v)
    } else {
        format!("${v:.2}")
    }
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
    match label.len() {
        16 => label[11..].to_string(),
        10 => label[5..].to_string(),
        _ => label.to_string(),
    }
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
        assert_eq!(usd(1.5), "$1.5");
        assert_eq!(usd(0.0), "$0.0");
        assert_eq!(usd(12.34), "$12.3");
        assert_eq!(usd(165.9051), "$165.9");
        assert_eq!(usd(1234.56), "$1234.6");
    }

    #[test]
    fn axis_costs_get_more_decimals_only_when_they_need_them() {
        assert_eq!(usd_axis(250.0), "$250");
        assert_eq!(usd_axis(12.5), "$12.5");
        assert_eq!(usd_axis(0.25), "$0.25");
        assert_eq!(usd_axis(0.0), "$0.00");
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
