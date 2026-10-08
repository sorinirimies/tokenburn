//! TokenBurn GUI — Iced desktop frontend for local AI token/cost usage.
//!
//! Built on top of [`tokenburn_core`] using the [Iced](https://iced.rs) toolkit
//! (Elm architecture: [`state`] → [`view`] → [`message`] → [`update`]).

#[macro_use]
mod macros;

pub mod args;
pub mod features;
pub mod message;
pub mod state;
pub mod update;
pub mod view;
pub mod widgets;
pub mod window;

pub use args::Args;
pub use message::Message;
pub use state::TokenBurn;
