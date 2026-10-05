//! Query + aggregation: turn raw [`Row`](crate::Row)s into report tables.

pub mod ops;
pub mod types;

pub use ops::{build_report, by_project};
pub use types::{Bucket, Query, ReportRow, ToolFilter, Window};
