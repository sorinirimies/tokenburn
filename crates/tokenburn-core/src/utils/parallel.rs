//! A tiny order-preserving parallel map on scoped threads (no extra dependency).
//!
//! A cold refresh parses gigabytes of transcripts; the per-file work is
//! independent, so files are spread over the available cores.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// `items.map(f)` on up to `available_parallelism` threads; results keep the
/// order of `items`. Falls back to a plain loop for a single item or core.
pub fn par_map<T, R>(items: Vec<T>, f: impl Fn(T) -> R + Sync) -> Vec<R>
where
    T: Send,
    R: Send,
{
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(items.len());
    if threads <= 1 {
        return items.into_iter().map(f).collect();
    }
    let n = items.len();
    let slots: Vec<Mutex<Option<T>>> = items.into_iter().map(|t| Mutex::new(Some(t))).collect();
    let results: Vec<Mutex<Option<R>>> = (0..n).map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= n {
                    break;
                }
                // A poisoned slot only means another worker panicked; the data is intact.
                let item = slots[i].lock().unwrap_or_else(|e| e.into_inner()).take();
                if let Some(item) = item {
                    let r = f(item);
                    *results[i].lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                }
            });
        }
    });
    results
        .into_iter()
        .filter_map(|m| m.into_inner().unwrap_or_else(|e| e.into_inner()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_order_and_runs_every_item() {
        let out = par_map((0..1000).collect(), |i| i * 2);
        assert_eq!(out, (0..1000).map(|i| i * 2).collect::<Vec<_>>());
    }

    #[test]
    fn empty_and_single_inputs_work() {
        assert!(par_map(Vec::<u8>::new(), |x| x).is_empty());
        assert_eq!(par_map(vec![7], |x| x + 1), vec![8]);
    }
}
