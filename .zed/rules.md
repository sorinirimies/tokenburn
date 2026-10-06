# TokenBurn — Agent Rules

## ⚠️ MANDATORY: Tests After Every Feature

**After implementing any feature, bug fix, or refactor, you MUST write tests before considering the task complete.**

- For **`tokenburn-core`**: add `#[cfg(test)] mod tests` inline in the changed file (`types.rs` or `ops.rs`).
- For **`tokenburn-tui`**: add `#[cfg(test)] mod tests` inline in the changed file (`app.rs`, `events.rs`, `features/<name>/events.rs`, etc.).
- For **`tokenburn-gui`**: add `#[cfg(test)] mod tests` inline in `state.rs` or the relevant feature file (`update.rs`, `view.rs`). Test state transitions and update logic; pure rendering is harder to test so focus on the update layer.
- **Run `cargo test` (or `cargo test --manifest-path ...`) after adding tests** and confirm they pass before finishing.
- If you realise tests are missing after the fact, write them immediately — do not move on to the next task without them.
- **Never respond "done" or summarise a completed feature without mentioning which tests were added.**

### Self-check before responding "done"

Before writing any closing summary, mentally run through this checklist and confirm every item:

1. **Happy-path test** — does at least one test exercise the normal/expected behaviour?
2. **Edge cases** — are boundary conditions covered (empty input, single item, max item, index 0, last index, out-of-bounds)?
3. **Error / noop paths** — are invalid or no-op inputs tested (e.g. closing the last tab, switching to an out-of-bounds index)?
4. **Regression guard** — is the exact bug scenario that was reported covered by a named test with a comment explaining the original failure?
5. **`cargo test` is green** — all tests pass with no warnings.

If any item is **No**, add the missing tests before finishing. Do not ask the user whether tests are needed — just add them.


## ⚠️ MANDATORY: Check Formatting After Every Change

**After every implementation, refactor, or fix — before considering the task complete — you MUST run:**

```
cargo fmt --all
```

Then verify there are no remaining issues with:

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Rules:
- **Never leave unformatted code.** If `cargo fmt --all` changes any file, the task is not done until those changes are committed or at least applied.
- **Clippy warnings are errors.** The CI treats `-D warnings` as hard failures. Fix every warning before finishing — do not suppress with `#[allow(...)]` unless there is a documented reason.
- **Run order**: `cargo fmt --all` → `cargo clippy` → `cargo test --lib` — in that order. A clean result on all three is the definition of "done".
- **Never respond "done" without confirming `cargo fmt --all` produced no diff and `cargo clippy` produced no warnings.**

## Project Overview

TokenBurn reports local AI token/cost usage for **pi** and **Zed**, read entirely from log files on disk (no network). It ships four front-ends — a clap **CLI**, a Ratatui **TUI**, an Iced **GUI** and a Topcoat **web** dashboard — from a single Cargo workspace. All data access and aggregation lives in the shared `tokenburn-core` crate — zero log parsing belongs in the UI layer.

```
tokenburn (cli) ──┐
tokenburn-tui ────┼──▶ tokenburn-core ──▶ rusqlite, zstd, serde_json, chrono
tokenburn-gui ────┤
tokenburn-web ────┘
```

The original single-file Nushell implementation is kept as `scripts/tokenburn.nu` (legacy reference).

## Workspace Layout

| Crate | Path | Purpose |
|---|---|---|
| `tokenburn-core` | `crates/tokenburn-core/` | Framework-free library: pi/Zed collectors, query + aggregation |
| `tokenburn` (CLI) | `crates/tokenburn-cli/` | clap CLI, binary `tokenburn` |
| `tokenburn-tui` | `crates/tokenburn-tui/` | Terminal UI built on Ratatui + Crossterm |
| `tokenburn-gui` | `crates/tokenburn-gui/` | Desktop GUI built on Iced (Elm Architecture) |
| `tokenburn-web` | `crates/tokenburn-web/` | Server-rendered web dashboard built on Topcoat (edition 2024) |

## Architecture Rules

### Core Crate (`tokenburn-core`)

- **Feature modules** live under `src/features/<name>/`: one collector per tool (`pi`, `zed`, `claude`, `codex`, `copilot`, `gemini`, `opencode`, `amp` — see "Adding a tool" in the core README), plus `usage` and `report` with `types.rs` (plain data, serde derives), `ops.rs` (logic returning `anyhow::Result<T>`) and `mod.rs` (re-exports).
- **Error handling**: `anyhow::Result` with `.context("descriptive message")`. A missing log source is *not* an error — return no rows. Per-source failures surface as `Collected::warnings`.
- **No UI code** in core. New data/aggregation functionality goes in core first.
- Collectors take explicit paths in `*_from` variants so they are testable with `tempfile`.

### CLI Crate (`tokenburn`)

- `args.rs` (clap derive) → `Query`; `render.rs` (pure table renderer); `lib.rs::run(args, out)` writes to any `Write` so it is testable.
- Keep flag compatibility with the Nushell script: `--tool`, `--hourly`, `--daily`, `--monthly`, `--all` (+ `--json`).

### GUI Crate (`tokenburn-gui`) — Elm Architecture

State → View → Message → Update. `TokenBurn::update(&mut self, msg) -> Task<Message>`; `TokenBurn::view(&self) -> Element<Message>`. Feature modules under `src/features/<name>/` hold `view.rs` / `commands.rs`. Blocking work runs off-thread via `blocking_task!` in `commands.rs`. Keep `Message` flat.

### Web Crate (`tokenburn-web`) — Topcoat

Server-rendered, **one flat crate** (no server/client split), one module per concern: `app` (`ServerOptions`, `router`, `serve`), `pages` (document shell, home page, live shard), `api` (`/api/report`, `/api/health`, `/style.css`), `components`, `params`, `data`, `style`. `main.rs` is thin (clap → `serve`). Routes use **absolute paths** (`#[page("/")]`, `#[route(GET "/...")]`) registered via `.discover()` — no `module_router!`, so file layout never changes URLs.

**Live mode**: when `AssetBundle::load()` succeeds (`just run-web` bundles into `target/debug/assets`), `app::router` enables `.assets().runtime()`; the `live_dashboard` `#[shard]` holds the filters as `signal`s (read on the server → only the shard re-renders) and a `live!` region that re-emits fresh data every `REFRESH_EVERY` while `connected(cx)`. **Static fallback** (no bundle): GET form + `<meta refresh>`; the `Interactive(bool)` app context picks the path. Signal values come from the browser — always pass them through `params::parse`. Binds `127.0.0.1` by default and opens the browser on start (`--no-open` to skip). Needs `edition = "2024"` (Topcoat macros rely on the 2024 prelude).

### TUI Crate (`tokenburn-tui`) — Immediate-Mode Loop

Use the author's own widget crates wherever one fits: `tui-spinner` (loading), `tui-piechart` (share chart), `tui-checkbox` + `tui-slider` (options popup), `tui-file-explorer` if a file picker is ever needed. Quirk: `tui-slider` draws a horizontal slider's label/value on the row *above* the area it is given.

~30 fps ratatui loop. Loading runs on a background thread and reports through `mpsc` (`bg_tx`/`bg_rx` on `App`); `poll_background()` drains it every tick. Stale results (query changed meanwhile) are dropped. Feature modules under `src/features/<name>/` hold `view.rs` / `events.rs`.

## Releases and nightly dependency updates

- GitHub (`origin`) and Gitea (`gitea-starscream`) are **independent hosts — no mirroring**. Each runs its own nightly `deps-update.yml` (GitHub 02:00 UTC, Gitea 02:30 UTC): `cargo update` → fmt/clippy/tests (GitHub: also macOS + Windows) → MSRV check → commit lock → bump patch → tag → push straight to `main` → start `release.yml` (all platforms + crates.io). No changes ⇒ nothing happens. If the histories diverge, force-push the one to keep (`just push-all-force`).
- First release: `just release-initial` (tags the version already in `Cargo.toml`). Later: `just release <version>`.
- Secrets: `CRATES_IO_TOKEN` (publish), optional `GH_PAT` (GitHub: tag push triggers release.yml itself), `GITEA_TOKEN` (Gitea nightly push/dispatch).

## Coding Standards

- **Rust edition**: 2021, MSRV 1.80+.
- **Formatting**: `cargo fmt --all` — no custom rustfmt config.
- **Linting**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`. Warnings are errors in CI.
- **Dependencies**: Pin versions in `[workspace.dependencies]` in the root `Cargo.toml`. Each crate references them with `{ workspace = true }`.
- Prefer `just check-all` before committing (runs fmt + clippy + test + nu tests).

## Testing Patterns (see also the mandatory rule at the top of this file)

- **Core**: inline `#[cfg(test)] mod tests`; use `tempfile` for fake pi sessions / a throwaway sqlite `threads.db`.
- **CLI**: parse with `Args::parse_from([...])`, assert on `Query`; test `render::table` output as a string.
- **Web**: unit-test `params::parse`, `data::build_dashboard`; smoke-test routes by running `tokenburn-web` and `curl`ing `/`, `/api/report`.
- **GUI**: call `TokenBurn::update()` directly and assert on state; no Iced runtime needed.
- **TUI**: build an `App`, call `handle_key()`, and render into `ratatui::backend::TestBackend` for smoke tests.
- **Naming**: `<thing_under_test>_<scenario>_<expected_outcome>`.

## Dev Workflow (`justfile`)

| Command | Purpose |
|---|---|
| `just build` | Build workspace |
| `just run-cli --daily` | Run the CLI |
| `just run-gui` | Run GUI |
| `just run-web` | Serve the web dashboard |
| `just run-tui` | Run TUI |
| `just test` | Run all tests |
| `just check-all` | fmt + clippy + test + nu tests |
| `just fmt` | Format all code |
| `just clippy` | Lint with `-D warnings` |
| `just release 0.X.Y` | Bump, tag, push (runs all checks) |
| `just package-linux 0.X.Y` | Build .deb + .rpm for Linux |
| `just package-appimage 0.X.Y` | Build AppImage for Linux |
| `just package-macos 0.X.Y` | Build universal DMG for macOS |
| `just update-aur 0.X.Y` | Update AUR PKGBUILD with new version |

## Common Pitfalls

1. **Don't skip tests.** Every feature implementation must be accompanied by tests.
2. **Don't skip formatting.** `cargo fmt --all` and clippy with `-D warnings` after every change.
3. **Don't put log parsing / aggregation in the UI crates.** Add it to `tokenburn-core` first.
4. **Don't block the UI thread.** GUI uses `Task` + `blocking_task!`; TUI uses `mpsc` background threads.
5. **Don't add dependencies to individual crate `Cargo.toml` directly.** Add them to `[workspace.dependencies]` first.
6. **Don't use `thiserror` for new errors.** Stick with `anyhow::Result` + `.context()`.
7. **Match existing module structure.** Every feature follows the same file layout.
8. **Keep the `Message` enum flat** in the GUI.
9. **GIF/PNG demo assets** (`examples/vhs/generated/`) are tracked with Git LFS (`.gitattributes`). TUI/CLI: `just vhs-tui` (tapes in `examples/vhs/`); GUI/web: `just screenshots` (`examples/capture/`). Both run on the synthetic `examples/vhs/fixture.sh` data, never real logs.
10. **Never hard-code colours in a front-end.** TUI views read `app.theme` (`UiTheme`), the GUI builds a custom iced `Theme` from the core palette, the web uses CSS variables from `theme::css`. Themes live in `tokenburn-core::features::theme`; a new colour slot goes into `AppTheme` and every theme. Tests must use `App::new()` / `TokenBurn::new()` (persistence off) so they never write the user's `settings.json`.
11. **Cross-platform is a requirement** (macOS, Linux, Windows). Never hard-code `/` paths or `~`: use `dirs` and `PathBuf::join`, and keep log-location logic in pure functions (`resolve_sessions_dir`, `candidates_from`) that tests can feed fake environments. Anything platform-specific goes behind `#[cfg(target_os = …)]`; check with `cargo check --target <triple>` (zig can stand in for the C cross-compiler).
12. **Linux GUI must stay `dlopen`-based** — keep iced's default `x11` + `wayland` + `wgpu` + `tiny-skia` features. Don't add crates that link X11/Wayland/GL at build time (it would break "one binary, any distro, X11 and Wayland"). The window `application_id` must equal the `.desktop` file basename.
13. **Icons are generated** by `python3 scripts/gen_icons.py` and committed. The Windows `.ico` must use classic BMP entries (NSIS rejects PNG-compressed ones).
14. **Web asset IDs don't depend on the compile target** but do depend on source paths, so build the web runtime bundle (`scripts/ci/bundle_web.sh`) in the same CI job/checkout as the binary it ships with.
15. **Zed has no `$` cost** locally — Zed rows always show cost 0; don't "fix" that.
16. **Interactive charts**: selection logic lives in `tokenburn-core::drill` (`Selection`, `drill`) and is shared by the GUI and web — never re-implement filtering in a front-end. GUI canvas programs publish `BucketClicked` / `SliceClicked`; hit-testing (`bar_at`, `slice_at`, `legend_row_at`) is pure and unit-tested. Web geometry (`charts.rs`) is pure too; in live mode a click sets a Topcoat signal, in static mode bars/slices are links (`?sel_bucket=…&sel_project=…`). No manual refresh controls anywhere: everything self-updates.
17. **Conventional commits feed the changelog** (git-cliff, `cliff.toml`). Use `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore` (scope optional: `feat(gui): …`); `gui:`/`tui:`/`web:`/`cli:`/`core:` prefixes land under "Front-ends", `release:` under "CI", `chore(deps):` under "Dependencies". Version bumps are skipped. Anything else is silently dropped from the changelog. Preview with `just changelog-preview`; `just bump` regenerates `CHANGELOG.md`.
