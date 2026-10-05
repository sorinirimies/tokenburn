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

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::app::App;
pub use crate::args::Args;
use tokenburn_core::Query;

/// Run the TUI until the user quits, starting from `query`.
pub fn run(query: Query) -> Result<()> {
    // `ratatui::init` enables raw mode + alternate screen and installs a
    // panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, query);
    ratatui::restore();
    result
}

/// The inner event loop — draw, poll for input, dispatch, repeat.
pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, query: Query) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut app = App::new();
    app.query = query;
    app.reload();

    while !app.should_quit {
        app.tick_count = app.tick_count.wrapping_add(1);
        app.poll_background();
        app.tick_auto_refresh();

        terminal.draw(|frame| layout::render(&mut app, frame))?;

        // ~30 fps so background results and the spinner feel prompt.
        if event::poll(Duration::from_millis(33))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    events::handle_key(&mut app, key);
                }
            }
        }
    }
    Ok(())
}
