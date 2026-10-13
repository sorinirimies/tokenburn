#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|s: &str| {
    let _ = serde_json::from_str::<tokenburn_core::Settings>(s);
    let _ = tokenburn_core::theme_index_by_name(s);
    let _ = tokenburn_core::find_theme(s);
});
