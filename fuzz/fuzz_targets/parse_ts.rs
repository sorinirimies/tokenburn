#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|s: &str| {
    let _ = tokenburn_core::utils::time::parse_ts(s);
});
