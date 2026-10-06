use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::features::usage::{Row, Tool};
use crate::utils::cache::{FileCache, Sig};
use crate::utils::files::{modified_since, num_any, read_lines, split_paths, walk_ext};
use crate::utils::time::parse_ts;

/// `$GEMINI_DATA_DIR` (list) or `~/.gemini/tmp`.
pub fn roots() -> Vec<PathBuf> {
    match std::env::var("GEMINI_DATA_DIR")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        Some(list) => split_paths(&list),
        None => dirs::home_dir()
            .map(|h| vec![h.join(".gemini").join("tmp")])
            .unwrap_or_default(),
    }
}

pub fn collect_gemini(start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for root in roots().into_iter().filter(|r| r.is_dir()) {
        rows.extend(collect_gemini_from(&root, start)?);
    }
    Ok(rows)
}

/// Parsed rows per chat recording, reused while the file is unchanged.
static CACHE: LazyLock<FileCache<Vec<Row>>> = LazyLock::new(FileCache::default);

/// Read every chat recording (`<project>/chats/*.json` and `*.jsonl`).
pub fn collect_gemini_from(root: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    let files = walk_ext(root, &["json", "jsonl"]);
    let live: HashSet<PathBuf> = files.iter().cloned().collect();
    let mut rows = Vec::new();
    for file in files {
        if !file.components().any(|c| c.as_os_str() == "chats") || !modified_since(&file, start) {
            continue;
        }
        let Some(sig) = Sig::of(&file, 0) else {
            continue;
        };
        let parsed = CACHE.get_or_parse(&file, sig, || parse_file(root, &file, start));
        rows.extend(parsed.iter().cloned());
    }
    CACHE.prune_under(root, &live);
    Ok(rows)
}

fn parse_file(root: &Path, file: &Path, start: DateTime<Utc>) -> Vec<Row> {
    let project = project_of(root, file);
    let id = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Messages without their own timestamp fall back to the file's mtime — a
    // value that changes whenever the file does, i.e. exactly when the cache entry
    // is invalidated anyway.
    let fallback = std::fs::metadata(file)
        .and_then(|m| m.modified())
        .ok()
        .map(DateTime::<Utc>::from)
        .unwrap_or(start);
    if file.extension().is_some_and(|e| e == "json") {
        std::fs::read_to_string(file)
            .map(|text| parse_json(&text, &project, &id, fallback))
            .unwrap_or_default()
    } else {
        read_lines(file)
            .map(|lines| {
                lines
                    .filter_map(|l| {
                        parse_message(&serde_json::from_str(&l).ok()?, &project, &id, fallback)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// `<root>/<project-hash-or-name>/chats/x.json` → the directory below `root`.
fn project_of(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .ok()
        .and_then(|rel| rel.components().next())
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn parse_json(text: &str, project: &str, id: &str, fallback: DateTime<Utc>) -> Vec<Row> {
    let Ok(doc) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let session_ts = ["startTime", "lastUpdated"]
        .iter()
        .find_map(|k| doc.get(*k).and_then(Value::as_str).and_then(parse_ts))
        .unwrap_or(fallback);
    doc.get("messages")
        .and_then(Value::as_array)
        .map(|msgs| {
            msgs.iter()
                .filter_map(|m| parse_message(m, project, id, session_ts))
                .collect()
        })
        .unwrap_or_default()
}

/// One `type: "gemini"` message carrying a `tokens` object.
fn parse_message(m: &Value, project: &str, id: &str, fallback: DateTime<Utc>) -> Option<Row> {
    if m.get("type").and_then(Value::as_str) != Some("gemini") {
        return None;
    }
    let t = m.get("tokens").filter(|t| t.is_object())?;
    let cached = num_any(t, &["cached", "cached_tokens"]);
    // Gemini's input count includes the cached portion: split it out.
    let input =
        num_any(t, &["input", "prompt", "input_tokens", "prompt_tokens"]).saturating_sub(cached);
    // Thinking tokens are billed as output.
    let output = num_any(
        t,
        &["output", "candidates", "output_tokens", "candidates_tokens"],
    ) + num_any(
        t,
        &[
            "thoughts",
            "reasoning",
            "thoughts_tokens",
            "reasoning_tokens",
        ],
    );
    if input + output + cached == 0 {
        return None;
    }
    let ts = m
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_ts)
        .unwrap_or(fallback);
    Some(Row {
        tool: Tool::Gemini,
        project: m
            .get("model")
            .and_then(Value::as_str)
            .map(|model| format!("{project}:{model}"))
            .unwrap_or_else(|| project.to_string()),
        id: id.to_string(),
        ts,
        input,
        output,
        cache_read: cached,
        cache_write: 0,
        cost: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SESSION: &str = r#"{"sessionId":"s1","startTime":"2026-05-01T10:00:00Z","messages":[
      {"type":"user","content":"hi"},
      {"type":"gemini","timestamp":"2026-05-01T10:00:05Z","model":"gemini-x",
       "tokens":{"input":100,"output":20,"cached":30,"thoughts":5,"tool":0,"total":155}},
      {"type":"gemini","tokens":{"input":0,"output":0,"cached":0}}]}"#;

    #[test]
    fn splits_cached_out_of_input_and_adds_thoughts_to_output() {
        let r = parse_json(SESSION, "proj", "s1", DateTime::<Utc>::UNIX_EPOCH);
        assert_eq!(
            r.len(),
            1,
            "user messages and all-zero messages are skipped"
        );
        assert_eq!((r[0].input, r[0].output, r[0].cache_read), (70, 25, 30));
        assert_eq!(r[0].project, "proj:gemini-x");
        assert_eq!(r[0].tool, Tool::Gemini);
    }

    #[test]
    fn accepts_the_alias_key_names() {
        let m = json!({"type":"gemini","tokens":{"prompt":10,"candidates":4,"cached_tokens":2,"reasoning":1}});
        let r = parse_message(&m, "p", "i", DateTime::<Utc>::UNIX_EPOCH).unwrap();
        assert_eq!((r.input, r.output, r.cache_read), (8, 5, 2));
    }

    #[test]
    fn only_files_under_a_chats_directory_are_read() {
        let d = tempfile::tempdir().unwrap();
        let chats = d.path().join("abc123/chats");
        std::fs::create_dir_all(&chats).unwrap();
        std::fs::write(chats.join("session-1.json"), SESSION).unwrap();
        std::fs::write(d.path().join("abc123/settings.json"), SESSION).unwrap();
        let r = collect_gemini_from(d.path(), DateTime::<Utc>::UNIX_EPOCH).unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn jsonl_recordings_are_supported() {
        let d = tempfile::tempdir().unwrap();
        let chats = d.path().join("p/chats");
        std::fs::create_dir_all(&chats).unwrap();
        std::fs::write(
            chats.join("s.jsonl"),
            r#"{"type":"gemini","timestamp":"2026-05-01T10:00:00Z","tokens":{"input":5,"output":1}}
not json
"#,
        )
        .unwrap();
        assert_eq!(
            collect_gemini_from(d.path(), DateTime::<Utc>::UNIX_EPOCH)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn garbage_documents_are_ignored() {
        assert!(parse_json("{not json", "p", "i", DateTime::<Utc>::UNIX_EPOCH).is_empty());
    }
}
