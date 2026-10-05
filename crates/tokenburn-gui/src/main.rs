use std::sync::OnceLock;

use clap::Parser;
use iced::{Settings, Task};
use tokenburn_core::Query;
use tokenburn_gui::{Args, Message, TokenBurn};

/// Initial query from the command line (read once by `boot`).
static INITIAL: OnceLock<Query> = OnceLock::new();

fn main() -> iced::Result {
    // Handles --help / --version before any window is created.
    let args = Args::parse();
    let _ = INITIAL.set(args.query());

    tracing_subscriber::fmt::init();

    iced::application(boot, TokenBurn::update, TokenBurn::view)
        .title("TokenBurn — AI token usage")
        .theme(|state: &TokenBurn| state.iced_theme())
        .subscription(subscription)
        .settings(Settings {
            default_text_size: iced::Pixels(14.0),
            ..Default::default()
        })
        .window_size(iced::Size::new(1200.0, 760.0))
        .centered()
        .run()
}

/// Boot function — initial state plus the first load.
fn boot() -> (TokenBurn, Task<Message>) {
    let mut state = TokenBurn::with_query(INITIAL.get().copied().unwrap_or_default());
    let task = state.reload();
    (state, task)
}

fn subscription(state: &TokenBurn) -> iced::Subscription<Message> {
    // Slow auto-refresh so the numbers track live sessions.
    let auto_refresh = iced::Subscription::run(auto_refresh_stream);
    // Spinner animation only while a load is in flight.
    let animation = if state.is_loading {
        iced::Subscription::run(animation_tick_stream)
    } else {
        iced::Subscription::none()
    };
    iced::Subscription::batch([auto_refresh, animation])
}

/// Emits `Refresh` every 60 s. Blocking sleep is fine: Iced runs each
/// subscription on its own worker thread.
fn auto_refresh_stream() -> impl futures::Stream<Item = Message> {
    futures::stream::unfold((), |()| async {
        std::thread::sleep(std::time::Duration::from_secs(60));
        Some((Message::Refresh, ()))
    })
}

/// Emits `AnimationTick` at ~10 fps.
fn animation_tick_stream() -> impl futures::Stream<Item = Message> {
    futures::stream::unfold((), |()| async {
        std::thread::sleep(std::time::Duration::from_millis(100));
        Some((Message::AnimationTick, ()))
    })
}
