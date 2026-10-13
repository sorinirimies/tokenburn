#![no_main]
use libfuzzer_sys::fuzz_target;
use tokenburn_core::features::currency::ops;

fuzz_target!(|s: &str| {
    let _ = ops::find(s);
    let _ = ops::index_of(s);
    let _ = ops::resolve(s, &std::collections::BTreeMap::new());
});
