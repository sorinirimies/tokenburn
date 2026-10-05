//! Async command helpers for the report feature.

use iced::Task;
use tokenburn_core::{collect, Query};

use crate::message::Message;

/// Read the logs for `query` on a background thread.
pub(crate) fn load(query: Query) -> Task<Message> {
    blocking_task!(
        move |res| Message::Loaded(query, res),
        collect(query.tool, query.window.start(chrono::Local::now()))
    )
}
