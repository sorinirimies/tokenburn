//! Every event the UI can produce.

use tokenburn_core::{Bucket, Collected, Query, ToolFilter, Window};

use crate::features::theme::ThemeChoice;
use crate::state::View;

#[derive(Debug, Clone)]
pub enum Message {
    /// A bar was clicked (its bucket label).
    BucketClicked(String),
    /// A donut slice or legend row was clicked (the full `tool:project` key).
    SliceClicked(String),
    /// Drop the bucket/project selection.
    ClearSelection,
    /// A display frame, while a chart animation runs.
    Frame(std::time::Instant),
    /// Chart y-axis: tokens or cost.
    MetricSelected(tokenburn_core::Metric),
    /// Periodic tick: refresh unless a load is already in flight.
    AutoRefresh,
    /// Background load finished (the query it was started for + the result).
    Loaded(Query, Result<Collected, String>),
    ToolSelected(ToolFilter),
    BucketSelected(Bucket),
    WindowSelected(Window),
    ThemeSelected(ThemeChoice),
    ViewSelected(View),
    /// ~10 fps animation tick, only subscribed while loading.
    AnimationTick,
}
