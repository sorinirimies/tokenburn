//! Zed agent-thread collector (sqlite + zstd JSON blobs).

pub mod ops;

pub use ops::{collect_zed, collect_zed_from, db_candidates};
