//! Query + aggregation: turn raw [`Row`](crate::Row)s into report tables.

pub mod chart;
pub mod filter;
pub mod ops;
pub mod types;

pub use chart::{
    axis_label, axis_tokens, bucket_at, chart_bucket, nice_max, pie, short_label, short_name,
    slice_angles, time_series, Metric, Pie, Slice, TimeSeries,
};
pub use filter::RowFilter;
pub use ops::{build_report, by_project};
pub use types::{Bucket, Query, ReportRow, ToolFilter, Window};
