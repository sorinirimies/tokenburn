//! Timestamp helpers.

use chrono::{DateTime, NaiveDateTime, Utc};

/// Parse the timestamp flavours found in the agents' logs.
///
/// Accepts RFC 3339 (`2026-05-01T10:00:00Z`, with offset or fractional
/// seconds) and naive `YYYY-MM-DD[T ]HH:MM:SS[.f]` (interpreted as UTC).
pub fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(n.and_utc());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_z() {
        assert!(parse_ts("2026-05-01T10:00:00Z").is_some());
        assert!(parse_ts("2026-05-01T10:00:00.123+02:00").is_some());
    }

    #[test]
    fn naive_is_utc() {
        let a = parse_ts("2026-05-01 10:00:00").unwrap();
        let b = parse_ts("2026-05-01T10:00:00Z").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn garbage_is_none() {
        assert!(parse_ts("nope").is_none());
    }
}
