//! TokenBurn TUI — terminal user interface built with Ratatui.
//!
//! Powered by [`tokenburn_core`] for all data access and [`ratatui`] for
//! rendering. Call [`run`] to start the interactive report browser.

pub mod app;
pub mod args;
pub mod events;
pub mod features;
pub mod layout;
pub mod utils;
pub mod widgets;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::app::App;
pub use crate::args::Args;
use tokenburn_core::{Currency, Query, ThemeChoice};

/// Run the TUI until the user quits, starting from `query`.
///
/// `theme` (a preset or "System") overrides the saved theme;
/// `None` uses the one in the shared settings file.
pub fn run(query: Query, theme: Option<ThemeChoice>, currency: Option<Currency>) -> Result<()> {
    // `ratatui::init` enables raw mode + alternate screen and installs a
    // panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, query, theme, currency);
    ratatui::restore();
    result
}

/// The inner event loop — draw, poll for input, dispatch, repeat.
pub fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    query: Query,
    theme: Option<ThemeChoice>,
    currency: Option<Currency>,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut app = App::new();
    app.persist = true;
    app.query = query;
    match theme {
        // An explicit --theme is remembered.
        Some(ThemeChoice::System) => app.set_follow_system(),
        Some(ThemeChoice::Preset(i)) => app.set_theme(i),
        None => app.apply_theme_choice(tokenburn_core::settings::load().theme_choice()),
    }
    // The user's own exchange rates, then the currency: an explicit --currency is
    // remembered like --theme, otherwise the saved one is used.
    let settings = tokenburn_core::settings::load();
    app.input_price = settings.input_price();
    app.rates = settings.rates;
    match currency {
        Some(c) => app.set_currency(tokenburn_core::features::currency::index_of(c.code)),
        None => app.currency = tokenburn_core::settings::currency(),
    }
    app.reload();

    // Redraw only when something changed. An idle dashboard sleeps in `poll`
    // instead of repainting 30 times a second (which cost ~4 % of a core).
    let mut dirty = true;
    while !app.should_quit {
        dirty |= app.poll_background();
        if std::mem::take(&mut app.bell) {
            // A limit level was just crossed: ring the terminal bell.
            use std::io::Write as _;
            let _ = std::io::stdout().write_all(b"\x07");
            let _ = std::io::stdout().flush();
        }
        app.tick_auto_refresh();
        dirty |= app.refresh_system_theme();
        if app.is_loading {
            // Animate the spinner while a load is in flight.
            app.tick_count = app.tick_count.wrapping_add(1);
            dirty = true;
        }

        if dirty {
            terminal.draw(|frame| layout::render(&mut app, frame))?;
            dirty = false;
        }

        if event::poll(app.poll_timeout())? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    events::handle_key(&mut app, key);
                    dirty = true;
                }
                Event::Resize(..) => dirty = true,
                _ => {}
            }
        }
    }
    Ok(())
}
