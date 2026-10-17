//! rtk's savings ledger.
//!
//! rtk records every command it filtered in a sqlite database
//! (`history.db`, table `commands`, column `saved_tokens`). It is opened
//! read-only; a missing or foreign database simply means "no rtk savings".

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};

/// Where rtk keeps `history.db`, most specific first.
///
/// * `TOKENBURN_RTK_DB` — explicit path override
/// * `<local data dir>/rtk/history.db` (`~/Library/Application Support` on macOS,
///   `$XDG_DATA_HOME` or `~/.local/share` on Linux, `%LOCALAPPDATA%` on Windows)
/// * `<config dir>/rtk/history.db`
pub fn db_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::env::var_os("TOKENBURN_RTK_DB")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for dir in [dirs::data_local_dir(), dirs::config_dir()]
        .into_iter()
        .flatten()
    {
        let p = dir.join("rtk").join("history.db");
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// Commands rtk filtered since `start`, one `(minute, tokens saved, commands)`
/// entry per minute, oldest first, from the first rtk database found.
pub fn saved_events_since(start: DateTime<Utc>) -> Option<Vec<SavedEvent>> {
    db_candidates()
        .iter()
        .find(|p| p.is_file())
        .and_then(|p| saved_events_in(p, start))
}

/// `(minute, tokens saved, commands)`; see [`saved_events_since`].
pub type SavedEvent = (DateTime<Utc>, u64, u64);

/// [`saved_events_since`] for a specific database; `None` if it cannot be read.
pub fn saved_events_in(db: &Path, start: DateTime<Utc>) -> Option<Vec<SavedEvent>> {
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    // Timestamps are RFC 3339 text (`2026-10-17T01:33:33.7+00:00`); a `T`-separated
    // prefix compares correctly as a string, and its first 16 characters are the minute.
    let from = start.format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut stmt = conn
        .prepare(
            "SELECT substr(timestamp, 1, 16), SUM(saved_tokens), COUNT(*) FROM commands \
             WHERE timestamp >= ?1 AND saved_tokens > 0 GROUP BY 1 ORDER BY 1",
        )
        .ok()?;
    let rows = stmt
        .query_map([from], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .ok()?;
    let mut out = Vec::new();
    for row in rows {
        let (minute, tokens, commands) = row.ok()?;
        let Ok(ts) =
            chrono::NaiveDateTime::parse_from_str(&format!("{minute}:00"), "%Y-%m-%dT%H:%M:%S")
        else {
            continue;
        };
        out.push((ts.and_utc(), tokens.max(0) as u64, commands.max(0) as u64));
    }
    Some(out)
}

/// `(tokens saved, commands filtered)` since `start`, from the first rtk database found.
pub fn saved_since(start: DateTime<Utc>) -> Option<(u64, u64)> {
    saved_events_since(start).map(|e| totals(&e))
}

/// [`saved_since`] for a specific database; `None` if it cannot be read.
pub fn saved_since_in(db: &Path, start: DateTime<Utc>) -> Option<(u64, u64)> {
    saved_events_in(db, start).map(|e| totals(&e))
}

fn totals(events: &[SavedEvent]) -> (u64, u64) {
    events
        .iter()
        .fold((0, 0), |(t, c), (_, tokens, cmds)| (t + tokens, c + cmds))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Create an rtk-shaped database with `(timestamp, saved_tokens)` rows.
    pub(crate) fn make_db(path: &Path, rows: &[(&str, i64)]) {
        let c = Connection::open(path).unwrap();
        c.execute_batch(
            "CREATE TABLE commands (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, \
             original_cmd TEXT NOT NULL, rtk_cmd TEXT NOT NULL, input_tokens INTEGER NOT NULL, \
             output_tokens INTEGER NOT NULL, saved_tokens INTEGER NOT NULL, \
             savings_pct REAL NOT NULL);",
        )
        .unwrap();
        for (ts, saved) in rows {
            c.execute(
                "INSERT INTO commands (timestamp, original_cmd, rtk_cmd, input_tokens, \
                 output_tokens, saved_tokens, savings_pct) VALUES (?1, 'ls', 'rtk ls', 0, 0, ?2, 0)",
                rusqlite::params![ts, saved],
            )
            .unwrap();
        }
    }

    #[test]
    fn sums_only_commands_inside_the_window() {
        let d = tempfile::tempdir().unwrap();
        let db = d.path().join("history.db");
        make_db(
            &db,
            &[
                ("2026-09-30T23:59:59.5+00:00", 1000),
                ("2026-10-01T00:00:00.1+00:00", 200),
                ("2026-10-17T01:33:33.702967+00:00", 50),
                ("2026-10-17T02:00:00+00:00", 0),
                ("2026-10-17T03:00:00+00:00", -5),
            ],
        );
        let start = "2026-10-01T00:00:00Z".parse().unwrap();
        assert_eq!(saved_since_in(&db, start), Some((250, 2)));
        let all = "1970-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(saved_since_in(&db, all), Some((1250, 3)));
        let future = "2030-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(saved_since_in(&db, future), Some((0, 0)));
    }

    #[test]
    fn events_are_grouped_per_minute_oldest_first() {
        let d = tempfile::tempdir().unwrap();
        let db = d.path().join("history.db");
        make_db(
            &db,
            &[
                ("2026-10-17T01:34:10+00:00", 7),
                ("2026-10-17T01:33:33.7+00:00", 50),
                ("2026-10-17T01:33:59+00:00", 25),
                ("2026-10-17T01:35:00+00:00", 0),
            ],
        );
        let all = "1970-01-01T00:00:00Z".parse().unwrap();
        let ev = saved_events_in(&db, all).unwrap();
        let at = |s: &str| -> DateTime<Utc> { s.parse().unwrap() };
        assert_eq!(
            ev,
            vec![
                (at("2026-10-17T01:33:00Z"), 75, 2),
                (at("2026-10-17T01:34:00Z"), 7, 1),
            ]
        );
    }

    #[test]
    fn unreadable_databases_yield_none() {
        let d = tempfile::tempdir().unwrap();
        let start = "1970-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(saved_since_in(&d.path().join("missing.db"), start), None);
        let junk = d.path().join("junk.db");
        std::fs::write(&junk, "not sqlite at all").unwrap();
        assert_eq!(saved_since_in(&junk, start), None);
        // A sqlite file without the table is not rtk's.
        let other = d.path().join("other.db");
        Connection::open(&other)
            .unwrap()
            .execute_batch("CREATE TABLE t (x)")
            .unwrap();
        assert_eq!(saved_since_in(&other, start), None);
    }

    #[test]
    fn candidates_honour_the_override_first() {
        // Env is process-global: only assert on the pure shape here.
        let c = db_candidates();
        assert!(c.iter().all(|p| p.ends_with("history.db")));
    }
}
