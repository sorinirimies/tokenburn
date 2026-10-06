//! Query + aggregation: turn raw [`Row`](crate::Row)s into report tables.

pub mod filter;
pub mod ops;
pub mod types;

pub use filter::RowFilter;
pub use ops::{build_report, by_project};
pub use types::{Bucket, Query, ReportRow, ToolFilter, Window};
