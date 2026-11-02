---
name: tokenburn-rules
description: Rules for work in the tokenburn repository that the code can't show: constraints, ownership and must-keep behaviour. Use before changing, fixing, reviewing or extending code here, together with the rust-in-projects skill.
---

# tokenburn: rules the code can't show

Hard constraints for this repository. The general conventions (orientation, tests,
scripting, releasing) are in the `rust-in-projects` skill; this file holds only what is
specific to tokenburn.

* **Log parsing and aggregation live only in `tokenburn-core`.**
  * Chart selection (`drill`) and breakpoints (`LayoutClass`) are shared by all
    front-ends. Front-ends never re-implement filtering or hard-code column
    counts.
  * Themes are in `tokenburn-core::features::theme`. A new colour slot goes in
    `AppTheme` and in every theme.
* **Tests** use `App::new()` / `TokenBurn::new()` with persistence off, so they
  never write the user's `settings.json`.
* **Linux GUI stays `dlopen`-based**: keep iced's default `x11`, `wayland`, `wgpu`
  and `tiny-skia` features. Add nothing that links X11, Wayland or GL at build
  time.
  The window `application_id` equals the `.desktop` basename.
* Zed rows always cost $0. That's correct, don't "fix" it.
* **Web assets:** build the web runtime bundle in the same CI job and checkout as
  the binary it ships with, because asset IDs depend on source paths.
* **The nightly releases only on real upgrades.**
  * `scripts/ci/lock_changes.nu`: at least one shipped package higher, none
    lower.
  * Changing that logic means changing `test_lock_changes.nu` too.
* The Windows `.ico` uses BMP entries (NSIS rejects PNG ones).
* **Demo assets:** `examples/vhs/generated/` is in Git LFS. Regenerate with
  `just vhs-tui` / `just screenshots`, always from the synthetic fixture.
