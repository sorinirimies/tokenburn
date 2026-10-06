//! Filesystem + JSON helpers shared by the collectors.

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde_json::Value;

/// Recursively collect files below `root` whose extension is one of `exts`
/// (case-sensitive, without the dot). Unreadable directories are skipped.
pub fn walk_ext(root: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, exts, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, exts, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.contains(&e))
        {
            out.push(path);
        }
    }
}

/// `false` only when the file's mtime is known and older than `start` — cheap
/// pre-filter so untouched history is never parsed.
pub fn modified_since(path: &Path, start: DateTime<Utc>) -> bool {
    let start_sys: SystemTime = start.into();
    !matches!(
        fs::metadata(path).and_then(|m| m.modified()),
        Ok(m) if m < start_sys
    )
}

/// Stream the lines of a (possibly huge) file; invalid UTF-8 lines are skipped.
pub fn read_lines(path: &Path) -> Option<impl Iterator<Item = String>> {
    let f = File::open(path).ok()?;
    Some(BufReader::new(f).lines().map_while(Result::ok))
}

/// A JSON number as `u64`: accepts integers and floats, clamps negatives to 0.
pub fn num(v: Option<&Value>) -> u64 {
    match v {
        Some(Value::Number(n)) => n
            .as_u64()
            .or_else(|| n.as_f64().map(|f| f.max(0.0) as u64))
            .unwrap_or(0),
        _ => 0,
    }
}

/// An optional JSON number (already deserialised as `f64`) as `u64`: clamps
/// negatives and NaN to 0. Used by the typed per-line parsers.
pub fn num_f(v: Option<f64>) -> u64 {
    v.filter(|x| x.is_finite()).map_or(0, |x| x.max(0.0) as u64)
}

/// First of `keys` present in `obj` as a number (aliases across versions).
pub fn num_any(obj: &Value, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|k| obj.get(*k).filter(|v| v.is_number()))
        .map(|v| num(Some(v)))
        .unwrap_or(0)
}

/// A non-empty environment variable as a path.
pub fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Split a path-list environment value (`:` / `;` / `,` separated).
pub fn split_paths(value: &str) -> Vec<PathBuf> {
    value
        .split([',', ';', if cfg!(windows) { '\u{0}' } else { ':' }])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// `-Users-me-Projects-app-` → `Users-me-Projects-app`.
pub fn project_from_dirname(name: &str) -> String {
    name.trim_matches('-').to_string()
}

/// Last path component of a working directory (`/a/b/app` → `app`), or `None`.
pub fn dir_basename(cwd: &str) -> Option<String> {
    cwd.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn num_handles_ints_floats_negatives_and_junk() {
        assert_eq!(num(Some(&json!(7))), 7);
        assert_eq!(num(Some(&json!(7.9))), 7);
        assert_eq!(num(Some(&json!(-3))), 0);
        assert_eq!(num(Some(&json!("7"))), 0);
        assert_eq!(num(None), 0);
    }

    #[test]
    fn num_f_clamps_and_tolerates_junk() {
        assert_eq!(num_f(Some(7.9)), 7);
        assert_eq!(num_f(Some(-1.0)), 0);
        assert_eq!(num_f(Some(f64::NAN)), 0);
        assert_eq!(num_f(Some(f64::INFINITY)), 0);
        assert_eq!(num_f(None), 0);
    }

    #[test]
    fn num_any_takes_the_first_alias_present() {
        let v = json!({"prompt": 4, "input": 9});
        assert_eq!(num_any(&v, &["input", "prompt"]), 9);
        assert_eq!(num_any(&v, &["nope", "prompt"]), 4);
        assert_eq!(num_any(&v, &["nope"]), 0);
    }

    #[test]
    fn basename_handles_both_separators_and_trailing_slash() {
        assert_eq!(dir_basename("/a/b/app/").as_deref(), Some("app"));
        assert_eq!(dir_basename("C:\\Users\\me\\app").as_deref(), Some("app"));
        assert_eq!(dir_basename("").as_deref(), None);
    }

    #[test]
    fn project_dirname_trims_dashes() {
        assert_eq!(project_from_dirname("--home-me--"), "home-me");
    }

    #[test]
    fn split_paths_accepts_commas_and_semicolons() {
        assert_eq!(split_paths("/a, /b;/c").len(), 3);
        assert!(split_paths("").is_empty());
    }

    #[test]
    fn walk_ext_finds_nested_files_sorted() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("x/y")).unwrap();
        fs::write(d.path().join("x/y/a.jsonl"), "").unwrap();
        fs::write(d.path().join("b.jsonl"), "").unwrap();
        fs::write(d.path().join("c.txt"), "").unwrap();
        let files = walk_ext(d.path(), &["jsonl"]);
        assert_eq!(files.len(), 2);
        assert!(files.windows(2).all(|w| w[0] <= w[1]));
    }
}
