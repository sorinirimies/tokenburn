//! Zed agent-thread collector (sqlite + zstd JSON blobs).

pub mod ops;

pub use ops::{collect_zed, collect_zed_from, collect_zed_priced, db_candidates};
