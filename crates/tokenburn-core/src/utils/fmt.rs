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

/// `1.5` → `"$1.5000"`.
pub fn usd(v: f64) -> String {
    format!("${v:.4}")
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
        assert_eq!(usd(1.5), "$1.5000");
    }
}
