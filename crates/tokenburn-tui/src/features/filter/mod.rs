//! Filtering: `/` free-text filter and `f` per-tool checkboxes ([`tui_checkbox`]).
//!
//! Both work on the rows already loaded, so the report, chart, pie and totals
//! update as you type.

pub mod events;
pub mod view;
