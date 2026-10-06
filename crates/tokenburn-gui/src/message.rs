//! Every event the UI can produce.

use tokenburn_core::{Bucket, Collected, Query, ToolFilter, Window};

use crate::features::theme::ThemeChoice;
use crate::state::View;

#[derive(Debug, Clone)]
pub enum Message {
    /// Re-read the logs for the current query.
    Refresh,
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
