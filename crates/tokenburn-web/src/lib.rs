//! TokenBurn web — a server-rendered dashboard built with
//! [Topcoat](https://github.com/tokio-rs/topcoat).
//!
//! One flat crate, one module per concern:
//!
//! | module | what |
//! |---|---|
//! | [`app`] | [`ServerOptions`], router construction, [`serve`] |
//! | [`pages`] | the document shell, the home page and the live shard |
//! | [`charts`] | server-rendered SVG charts (stacked bars, donut) |
//! | [`api`] | JSON + static routes (`/api/report`, `/api/health`, `/style.css`) |
//! | [`components`] | reusable view components (cards, tables, filters) |
//! | [`params`] | query-string / signal values → core `Query` |
//! | [`data`] | loading and shaping the dashboard data |
//! | [`style`] | the stylesheet |
//! | [`theme`] | the core's colour themes as CSS custom properties |
//!
//! Routes use absolute paths and are registered through `.discover()`, so the
//! file layout never influences URLs.
//!
//! **Live mode** (asset bundle present): filters are signals and the data
//! refreshes itself over a WebSocket. **Static mode** (no bundle): GET-form
//! filters and a `<meta refresh>`.

pub mod api;
pub mod app;
pub mod charts;
pub mod components;
pub mod data;
pub mod pages;
pub mod params;
pub mod style;
pub mod theme;

pub use app::{ServerOptions, router, serve};
