//! Every event the UI can produce.

use tokenburn_core::{Bucket, Collected, Query, ToolFilter, Window};

use crate::features::currency::CurrencyChoice;
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
    /// Periodic re-check of the OS appearance, only subscribed while following it.
    SystemThemeTick,
    CurrencySelected(CurrencyChoice),
    ViewSelected(View),
    /// A limit check finished (limits, how much is used, new alerts).
    LimitsChecked(tokenburn_core::LimitCheck),
    /// The text of a limit field in Settings changed (US dollars; empty = off).
    LimitInput(tokenburn_core::Period, String),
    /// Enter pressed in a limit field: save the limits.
    LimitsApplied,
    /// Desktop notifications for limits on / off.
    LimitNotifyToggled(bool),
    /// Hide the alert banner.
    DismissAlert,
    /// ~10 fps animation tick, only subscribed while loading.
    AnimationTick,
}
