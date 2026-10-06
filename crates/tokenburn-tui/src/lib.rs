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
use tokenburn_core::Query;

/// Run the TUI until the user quits, starting from `query`.
///
/// `theme` (an index into [`tokenburn_core::THEME_NAMES`]) overrides the saved theme;
/// `None` uses the one in the shared settings file.
pub fn run(query: Query, theme: Option<usize>) -> Result<()> {
    // `ratatui::init` enables raw mode + alternate screen and installs a
    // panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, query, theme);
    ratatui::restore();
    result
}

/// The inner event loop — draw, poll for input, dispatch, repeat.
pub fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    query: Query,
    theme: Option<usize>,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut app = App::new();
    app.persist = true;
    app.query = query;
    match theme {
        Some(i) => app.set_theme(i), // an explicit --theme is remembered
        None => {
            app.theme_index = tokenburn_core::settings::theme_index();
            app.theme = crate::features::theme::UiTheme::by_index(app.theme_index);
        }
    }
    app.reload();

    // Redraw only when something changed. An idle dashboard sleeps in `poll`
    // instead of repainting 30 times a second (which cost ~4 % of a core).
    let mut dirty = true;
    while !app.should_quit {
        dirty |= app.poll_background();
        app.tick_auto_refresh();
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
