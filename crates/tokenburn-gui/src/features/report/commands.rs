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

/// Check the spending limits on a background thread. `live` logs the alerts and
/// sends desktop notifications; otherwise nothing is written or sent.
pub(crate) fn check_limits(live: bool, currency: tokenburn_core::Currency) -> Task<Message> {
    blocking_task!(
        |res| match res {
            Ok(check) => Message::LimitsChecked(check),
            Err(_) => Message::DismissAlert,
        },
        tokenburn_core::run_limit_check(live, |v| currency.format(v))
    )
}
