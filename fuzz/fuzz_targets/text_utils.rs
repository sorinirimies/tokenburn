#![no_main]
use libfuzzer_sys::fuzz_target;
use tokenburn_core::utils::{files, fmt};

fuzz_target!(|s: &str| {
    let _ = fmt::short_label(s);
    let _ = fmt::short_name(s);
    let _ = files::split_paths(s);
    let _ = files::project_from_dirname(s);
    let _ = files::dir_basename(s);
    let _ = tokenburn_core::appinfo::is_link(s);
});
