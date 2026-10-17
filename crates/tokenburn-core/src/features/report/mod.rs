//! Query + aggregation: turn raw [`Row`](crate::Row)s into report tables.

pub mod cards;
pub mod chart;
pub mod drill;
pub mod filter;
pub mod ops;
pub mod types;

pub use cards::{cards, Card, CardKind};
pub use chart::{
    axis_label, axis_tokens, bucket_at, bucket_label_at, bucket_label_of, chart_bucket, nice_max,
    pie, short_label, short_name, slice_angles, time_series, time_series_priced, Metric, Pie,
    SavedInput, Slice, TimeSeries,
};
pub use drill::{drill, drill_priced, project_key, Drill, Selection};
pub use filter::RowFilter;
pub use ops::{build_report, by_project, projects_add_detail};
pub use types::{Bucket, Query, ReportRow, ToolFilter, Window};
