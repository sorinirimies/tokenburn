//! A tiny per-file result cache so a refresh only re-parses what changed.
//!
//! Every collector parses one log file into rows. Re-reading a year of
//! transcripts every few seconds would burn CPU for no reason, so each file's
//! rows are cached against its *signature* (length + modification time): an
//! unchanged file is skipped entirely, a growing one is re-parsed. Idle
//! refreshes therefore cost a directory walk and a few `stat` calls.
//!
//! The cache lives for the life of the process (a TUI/GUI/web server keeps
//! refreshing); a short-lived CLI call never benefits and never pays.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// What identifies "this file, unchanged": size, mtime and a caller-chosen tag
/// (e.g. the window start for sqlite sources whose query depends on it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sig {
    len: u64,
    modified: Option<SystemTime>,
    tag: i64,
}

impl Sig {
    /// Signature of `path`, or `None` if it cannot be `stat`ed.
    pub fn of(path: &Path, tag: i64) -> Option<Sig> {
        let m = std::fs::metadata(path).ok()?;
        Some(Sig {
            len: m.len(),
            modified: m.modified().ok(),
            tag,
        })
    }

    /// Signature of several related files (a sqlite database and its `-wal`):
    /// sizes add up, the newest mtime wins.
    pub fn of_all(paths: &[PathBuf], tag: i64) -> Option<Sig> {
        let mut sig: Option<Sig> = None;
        for p in paths {
            let Some(s) = Sig::of(p, tag) else { continue };
            sig = Some(match sig {
                None => s,
                Some(a) => Sig {
                    len: a.len + s.len,
                    modified: a.modified.max(s.modified),
                    tag,
                },
            });
        }
        sig
    }
}

/// Parsed results per file, valid while the file's [`Sig`] is unchanged.
pub struct FileCache<T> {
    inner: Mutex<HashMap<PathBuf, (Sig, Arc<T>)>>,
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl<T> Default for FileCache<T> {
    fn default() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }
}

impl<T> FileCache<T> {
    /// The cached result for `path` if its signature still matches `sig`,
    /// otherwise run `parse` and remember the result.
    pub fn get_or_parse(&self, path: &Path, sig: Sig, parse: impl FnOnce() -> T) -> Arc<T> {
        if let Some((s, v)) = self.lock().get(path) {
            if *s == sig {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Arc::clone(v);
            }
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        // Parse outside the lock: other collectors/threads keep running.
        let value = Arc::new(parse());
        self.lock()
            .insert(path.to_path_buf(), (sig, Arc::clone(&value)));
        value
    }

    /// Forget files that no longer exist / are no longer in scope.
    pub fn prune(&self, keep: &HashSet<PathBuf>) {
        self.lock().retain(|p, _| keep.contains(p));
    }

    /// Forget cached files below `root` that are not in `keep` (deleted or out of
    /// scope), leaving every other root's entries alone.
    pub fn prune_under(&self, root: &Path, keep: &HashSet<PathBuf>) {
        self.lock()
            .retain(|p, _| !p.starts_with(root) || keep.contains(p));
    }

    /// `(hits, misses)` since creation — what tests (and `--bench`) look at.
    pub fn stats(&self) -> (usize, usize) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }

    /// Number of cached files.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, (Sig, Arc<T>)>> {
        // A panic while holding the lock only ever leaves a consistent map behind.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn unchanged_files_are_served_from_the_cache() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.jsonl");
        fs::write(&p, "one").unwrap();
        let cache: FileCache<String> = FileCache::default();
        let sig = Sig::of(&p, 0).unwrap();

        let mut parses = 0;
        for _ in 0..3 {
            let v = cache.get_or_parse(&p, sig, || {
                parses += 1;
                "parsed".to_string()
            });
            assert_eq!(*v, "parsed");
        }
        assert_eq!(parses, 1, "parsed once, then reused");
        assert_eq!(cache.stats(), (2, 1));
    }

    #[test]
    fn a_changed_file_is_parsed_again() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.jsonl");
        fs::write(&p, "one").unwrap();
        let cache: FileCache<usize> = FileCache::default();
        let a = cache.get_or_parse(&p, Sig::of(&p, 0).unwrap(), || 1);
        fs::write(&p, "one two three").unwrap(); // size changes
        let b = cache.get_or_parse(&p, Sig::of(&p, 0).unwrap(), || 2);
        assert_eq!((*a, *b), (1, 2));
        assert_eq!(cache.stats(), (0, 2));
    }

    #[test]
    fn the_tag_is_part_of_the_identity() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("db");
        fs::write(&p, "x").unwrap();
        let cache: FileCache<i64> = FileCache::default();
        let a = cache.get_or_parse(&p, Sig::of(&p, 100).unwrap(), || 100);
        let b = cache.get_or_parse(&p, Sig::of(&p, 200).unwrap(), || 200);
        assert_eq!(
            (*a, *b),
            (100, 200),
            "a different window start must not reuse rows"
        );
    }

    #[test]
    fn prune_forgets_files_that_left_scope() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("a"), d.path().join("b"));
        fs::write(&a, "x").unwrap();
        fs::write(&b, "y").unwrap();
        let cache: FileCache<u8> = FileCache::default();
        cache.get_or_parse(&a, Sig::of(&a, 0).unwrap(), || 1);
        cache.get_or_parse(&b, Sig::of(&b, 0).unwrap(), || 2);
        assert_eq!(cache.len(), 2);
        cache.prune(&HashSet::from([a]));
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
    }

    #[test]
    fn prune_under_only_touches_its_own_root() {
        let d = tempfile::tempdir().unwrap();
        let (r1, r2) = (d.path().join("r1"), d.path().join("r2"));
        fs::create_dir_all(&r1).unwrap();
        fs::create_dir_all(&r2).unwrap();
        let (a, b) = (r1.join("a"), r2.join("b"));
        fs::write(&a, "x").unwrap();
        fs::write(&b, "y").unwrap();
        let cache: FileCache<u8> = FileCache::default();
        cache.get_or_parse(&a, Sig::of(&a, 0).unwrap(), || 1);
        cache.get_or_parse(&b, Sig::of(&b, 0).unwrap(), || 2);
        cache.prune_under(&r1, &HashSet::new()); // r1 has no live files any more
        assert_eq!(cache.len(), 1, "r2's entry survives");
    }

    #[test]
    fn signatures_of_missing_files_are_none_and_groups_combine() {
        let d = tempfile::tempdir().unwrap();
        assert!(Sig::of(&d.path().join("nope"), 0).is_none());
        let (a, b) = (d.path().join("db"), d.path().join("db-wal"));
        fs::write(&a, "12345").unwrap();
        assert!(
            Sig::of_all(&[a.clone(), b.clone()], 0).is_some(),
            "the -wal may not exist"
        );
        let one = Sig::of_all(std::slice::from_ref(&a), 0).unwrap();
        fs::write(&b, "67").unwrap();
        let both = Sig::of_all(&[a, b], 0).unwrap();
        assert_ne!(one, both, "a growing -wal changes the group signature");
        assert!(Sig::of_all(&[], 0).is_none());
    }

    #[test]
    fn a_panic_in_parse_does_not_poison_the_cache() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a");
        fs::write(&p, "x").unwrap();
        let cache: Arc<FileCache<u8>> = Arc::new(FileCache::default());
        let c2 = Arc::clone(&cache);
        let p2 = p.clone();
        let _ = std::thread::spawn(move || {
            c2.get_or_parse(&p2, Sig::of(&p2, 0).unwrap(), || panic!("boom"));
        })
        .join();
        assert_eq!(*cache.get_or_parse(&p, Sig::of(&p, 0).unwrap(), || 7), 7);
    }
}
