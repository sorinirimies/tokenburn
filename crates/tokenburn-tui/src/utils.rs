//! Misc helpers.

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Braille spinner frame for a tick counter (advances every ~100 ms).
pub fn spinner(tick: u64) -> &'static str {
    SPINNER[((tick / 3) as usize) % SPINNER.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_wraps() {
        assert_eq!(spinner(0), SPINNER[0]);
        assert_eq!(spinner(3 * 10), SPINNER[0]);
    }
}
