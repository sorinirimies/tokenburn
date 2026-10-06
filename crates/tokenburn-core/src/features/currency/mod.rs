//! Display currencies. Costs are logged in US dollars; a [`Currency`] only changes
//! how they are shown, using fixed approximate rates the user can override in
//! `settings.json` (TokenBurn makes no network calls, so it cannot look rates up).

pub mod ops;
pub mod types;

pub use ops::{find, index_of, resolve};
pub use types::{Currency, CURRENCIES};
