use std::collections::HashMap;

/// What one million tokens of a model cost, in US dollars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    /// Tokens written to the prompt cache.
    pub cache_write: f64,
    /// Tokens read from the prompt cache.
    pub cache_read: f64,
}

impl Price {
    /// Cache write price when a table omits it: the 5-minute rate, 1.25 × input.
    pub const CACHE_WRITE_RATIO: f64 = 1.25;
    /// Cache read price when a table omits it: 0.1 × input.
    pub const CACHE_READ_RATIO: f64 = 0.1;

    /// Dollars for this many tokens of each kind.
    pub fn cost(&self, input: u64, output: u64, cache_write: u64, cache_read: u64) -> f64 {
        (input as f64 * self.input
            + output as f64 * self.output
            + cache_write as f64 * self.cache_write
            + cache_read as f64 * self.cache_read)
            / 1_000_000.0
    }
}

/// Prices by [normalized](super::normalize) model id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Prices {
    pub(crate) models: HashMap<String, Price>,
}

impl Prices {
    /// The price of `model`, however its name is spelled (see [`super::normalize`]).
    pub fn get(&self, model: &str) -> Option<&Price> {
        self.models.get(&super::normalize(model))
    }

    /// How many models have a price.
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// No model has a price.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}
