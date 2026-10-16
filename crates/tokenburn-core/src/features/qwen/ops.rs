use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::features::gemini::ops::collect_chats_from;
use crate::features::usage::{Row, Tool};
use crate::utils::files::split_paths;

/// `$QWEN_DATA_DIR` (list) or `~/.qwen/tmp`.
pub fn roots() -> Vec<PathBuf> {
    match std::env::var("QWEN_DATA_DIR")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        Some(list) => split_paths(&list),
        None => dirs::home_dir()
            .map(|h| vec![h.join(".qwen").join("tmp")])
            .unwrap_or_default(),
    }
}

pub fn collect_qwen(start: DateTime<Utc>) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for root in roots().into_iter().filter(|r| r.is_dir()) {
        rows.extend(collect_qwen_from(&root, start)?);
    }
    Ok(rows)
}

pub fn collect_qwen_from(root: &Path, start: DateTime<Utc>) -> Result<Vec<Row>> {
    collect_chats_from(Tool::Qwen, root, start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_tagged_qwen() {
        let d = tempfile::tempdir().unwrap();
        let chats = d.path().join("proj/chats");
        std::fs::create_dir_all(&chats).unwrap();
        std::fs::write(
            chats.join("s.json"),
            r#"{"startTime":"2026-05-01T10:00:00Z","messages":[
              {"type":"gemini","model":"qwen3-coder","tokens":{"input":10,"output":5,"cached":2}}]}"#,
        )
        .unwrap();
        let r = collect_qwen_from(d.path(), DateTime::<Utc>::UNIX_EPOCH).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].tool, Tool::Qwen);
        assert_eq!((r[0].input, r[0].cache_read), (8, 2));
    }
}
