<div align="center">

# 🔥 TokenBurn

**Local AI token & cost usage for pi, Zed, Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode and Amp — CLI, terminal UI, desktop GUI & web dashboard, all in Rust**

[![Crates.io](https://img.shields.io/crates/v/tokenburn.svg)](https://crates.io/crates/tokenburn)
[![docs.rs](https://docs.rs/tokenburn-core/badge.svg)](https://docs.rs/tokenburn-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

<img src="examples/vhs/generated/dashboard.gif" alt="TokenBurn TUI: report, stacked token chart and share-by-project pie" width="900">

</div>

---

TokenBurn reads **local logs only** — no network calls, no external service. It understands every agent harness and IDE that keeps token usage on disk:

| `--tool` | Tool | Where it reads (default) | Granularity | `$` cost |
|----------|------|--------------------------|-------------|:---:|
| `pi` | [pi](https://github.com/earendil-works/pi-coding-agent) | `~/.pi/agent/sessions/**/*.jsonl` | per assistant turn | ✅ |
| `zed` | Zed agent panel | `threads.db` (sqlite + zstd JSON) | per thread, input/output only | – |
| `claude` | Claude Code | `~/.claude/projects/**/*.jsonl` (+ `~/.config/claude`, `$CLAUDE_CONFIG_DIR`) | per message, de-duplicated | when logged |
| `codex` | OpenAI Codex CLI | `~/.codex/sessions/**/*.jsonl` (`$CODEX_HOME`) | per model call (deltas of the running totals) | – |
| `copilot` | GitHub Copilot CLI | `~/.copilot/session-state` (`$COPILOT_HOME`) | per session & model (`session.shutdown`) | – |
| `gemini` | Google Gemini CLI | `~/.gemini/tmp/*/chats` (`$GEMINI_DATA_DIR`) | per message | – |
| `opencode` | OpenCode | `~/.local/share/opencode/opencode.db` + legacy JSON (`$OPENCODE_DATA_DIR`) | per message | ✅ |
| `amp` | Amp | `~/.local/share/amp/threads/*.json` (`$AMP_DATA_DIR`) | per message | – |

`all` (the default) reads every tool that is installed; a tool that isn't simply contributes nothing. Each tool is read on its own thread. The `$` column says whether the tool logs a cost; where it doesn't, cost shows `$0.0000` rather than a guess (token counts are exact). Cursor, Windsurf, VS Code Copilot Chat and JetBrains AI keep no usable token log on disk, so they can't be supported.

> The Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode and Amp readers follow the on-disk formats documented by [ccusage](https://github.com/ryoppippi/ccusage) and are covered by fixture tests; pi and Zed were also checked against real logs. Open an issue with a (redacted) log line if one of them misreads your data.

One Rust workspace, four front-ends sharing one core:

| Binary | Crate | Use case |
|--------|-------|----------|
| `tokenburn` | `tokenburn` | clap CLI — scripts, pipes, `--json` |
| `tokenburn-tui` | `tokenburn-tui` | Ratatui terminal UI — interactive report browser |
| `tokenburn-gui` | `tokenburn-gui` | Iced desktop GUI — pickers, themes, share bars |
| `tokenburn-web` | `tokenburn-web` | [Topcoat](https://github.com/tokio-rs/topcoat) web dashboard — server-rendered HTML + JSON API |

No `python3` or `zstd` binary needed (sqlite and zstd are bundled).

## Always up to date

Every front-end refreshes itself — no key to press:

| Front-end | Refresh | Notes |
|-----------|---------|-------|
| TUI | every **5 s** (adjustable 5–300 s, or off, in the `o` popup) | the status line shows `updated 12:03:04 · 1,203 of 3,029 rows · auto-refresh 5s` |
| GUI | every **10 s** | the footer shows the last update time; ticks never pile up while a load is running |
| Web | every **5 s** over a WebSocket (live mode), every 10 s via meta-refresh (static mode) | the footer shows `● live` and the update time |

![auto-update: a new turn is logged and the numbers change by themselves](examples/vhs/generated/live-refresh.gif)

A refresh re-parses only the files that changed (see [Performance](#performance)), so leaving a dashboard open is cheap.

## Charts

Every front-end draws the same two charts from the same data (`tokenburn-core::chart`):

- **Tokens / cost over time** — one stacked bar per hour, day or month (follows the bucket you picked), one colour per tool, gap-filled so quiet days show as gaps, not as a squashed axis. Switch the y-axis between tokens and cost.
- **Share by project** — a donut of the biggest projects (the tail folds into "other"), with the total in the middle.

| Front-end | How it is drawn |
|-----------|-----------------|
| TUI | Ratatui bar chart + [`tui-piechart`](https://crates.io/crates/tui-piechart); toggle each panel in `o` |
| GUI | Iced canvas — hover a bar for a tooltip; the tokens/cost pick list sits above the chart |
| Web | server-rendered inline SVG (no JavaScript charting library); hover for native tooltips; `?metric=cost` or the *chart* select |

Chart colours come from the active theme, so all 43 themes restyle them.

| GUI | Web |
|-----|-----|
| ![GUI](examples/vhs/generated/gui.gif) | ![Web](examples/vhs/generated/web.gif) |

## Performance

TokenBurn is meant to sit open all day, so it avoids re-doing work. Measured on an Apple-silicon Mac, 2,600 turns across 8 tools (`just bench`):

| | Time | Memory |
|---|---|---|
| This month, warm refresh | ~20 ms | ~19 MB |
| All history, first read (cold) | ~480 ms | ~160–290 MB peak |
| All history, every refresh after that | ~20 ms | — |
| TUI idle (auto-refresh on) | < 1 % of a core | ~60 MB |
| Web server with no browser open | 0 % | ~25 MB |

How: every log file is parsed once and cached by `(length, mtime)`, so a refresh re-parses only the files that changed; Zed's compressed threads are cached per thread; the TUI redraws only when something changed and otherwise sleeps in `poll`; the web server does no work without a connected page.

## Filtering (TUI)

Filters apply instantly to the rows already loaded — totals, report, bar chart and pie all follow as you type:

| Key | Filter |
|-----|--------|
| `/` | free text: `tokenburn claude` keeps rows matching **all** words (tool, project/model or session id, case-insensitive); `-zed` excludes. `Enter` keeps it, `Esc` clears it |
| `f` | per-tool checkboxes with row counts — `␣` toggle, `a` all, `n` none, `x` clear every filter |
| `Esc` | clears an active filter first; a second `Esc` quits |

The header shows what is active, e.g. `⌕ filter:"tokenburn"  tools:3/8  412/3,029 rows`.

## Themes

43 colour themes — Dracula, Nord, Tokyo Night, Catppuccin (all four), Gruvbox, Solarized, Kanagawa, Rosé Pine, Everforest, Flexoki, Oxocarbon, Synthwave… dark **and** light — shared by the TUI, the GUI and the web dashboard. The palettes live once in `tokenburn-core` (shared with the author's GitKraft), so a theme looks the same everywhere, including one colour per tool in the charts, pie and bars.

| Front-end | Pick a theme |
|-----------|--------------|
| TUI | `T` opens a picker with live preview and swatches; `[` / `]` cycle; `--theme "Tokyo Night"` |
| GUI | the theme drop-down in the toolbar; `--theme NAME` |
| Web | the theme select (applies instantly in live mode); `?theme=Nord`; `--theme NAME`; palettes as JSON at `/api/themes` |

`--list-themes` prints every name. The choice is saved in `settings.json` (`~/.config/tokenburn/`, `~/Library/Application Support/tokenburn/`, `%APPDATA%\tokenburn\`; override the directory with `TOKENBURN_CONFIG_DIR`), so changing it in one front-end changes the starting theme of the others.

## Platform support

TokenBurn runs on **macOS, Linux and Windows**. The log locations are found per platform, and every one can be overridden with an environment variable:

| Source | macOS | Linux | Windows | Override |
|--------|-------|-------|---------|----------|
| pi sessions | `~/.pi/agent/sessions` | `~/.pi/agent/sessions` | `%USERPROFILE%\.pi\agent\sessions` | `TOKENBURN_PI_SESSIONS`, or pi's own `PI_CODING_AGENT_SESSION_DIR` / `PI_CODING_AGENT_DIR` |
| Zed threads | `~/Library/Application Support/Zed/threads/threads.db` | `$XDG_DATA_HOME/zed/…` (default `~/.local/share/zed/…`), Flatpak `~/.var/app/dev.zed.Zed/…` | `%LOCALAPPDATA%\Zed\threads\threads.db` | `TOKENBURN_ZED_DB` |

| Binary | macOS (arm64 + x86_64) | Linux (x86_64 + aarch64) | Windows (x86_64) |
|--------|:---:|:---:|:---:|
| `tokenburn` (CLI) | ✅ | ✅ | ✅ |
| `tokenburn-tui` | ✅ | ✅ | ✅ |
| `tokenburn-web` | ✅ | ✅ | ✅ |
| `tokenburn-gui` | ✅ `.app` in a universal DMG | ✅ X11 **and** Wayland | ✅ |

### Linux: any distro, X11 and Wayland

- **One GUI binary serves both X11 and Wayland.** It links only libc/libm; X11, Wayland, xkbcommon and the GPU libraries are loaded at runtime. It picks Wayland when `WAYLAND_DISPLAY` is set and X11 otherwise. Force X11 under a Wayland session (XWayland) with `env -u WAYLAND_DISPLAY tokenburn-gui`.
- **Rendering falls back to software** when there is no usable GPU/Vulkan (VMs, remote desktops). Force it with `ICED_BACKEND=tiny-skia tokenburn-gui`.
- The window sets its Wayland `app_id` / X11 `WM_CLASS` to `tokenburn-gui`, matching the installed `tokenburn-gui.desktop`, so panels show the right icon.
- **glibc builds** (`*-unknown-linux-gnu`, AppImage, `.deb`, `.rpm`) are linked on Ubuntu 22.04, so they need glibc ≥ 2.35 (Debian 12+, Ubuntu 22.04+, Fedora, Arch, openSUSE, …).
- **Fully static builds** (`x86_64-unknown-linux-musl`: CLI, TUI and web) have no glibc requirement at all — they run on Alpine, old enterprise distros, containers, NixOS, …
- The GUI needs `libxkbcommon` at runtime; a Wayland or X11 session library and fonts are expected as on any desktop Linux.

## Installation

### Prebuilt (GitHub / Gitea releases)

| Platform | Artifacts |
|----------|-----------|
| macOS | `tokenburn-<v>-macos.dmg` — `TokenBurn.app` + `Command Line Tools/` (CLI, TUI, web) |
| Windows | `tokenburn-<v>-windows-x86_64-setup.exe` (installer, optional PATH entry) or `…-x86_64-pc-windows-msvc.zip` |
| Linux | `.deb` / `.rpm` per app, `tokenburn-{tui,gui}-<v>-<arch>.AppImage`, and portable `.tar.gz` for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` and static `x86_64-unknown-linux-musl` |

Every portable archive holds all binaries built for that target, the web runtime in `assets/` (so `tokenburn-web` runs live), the README and the licence, plus a `.sha256`.

macOS note: the app is ad-hoc signed, not notarised — right-click → Open the first time, or `xattr -dr com.apple.quarantine TokenBurn.app`.

### From crates.io

```sh
cargo install tokenburn        # CLI
cargo install tokenburn-tui    # Terminal UI
cargo install tokenburn-gui    # Desktop GUI
cargo install tokenburn-web    # Web dashboard (static mode — see below for live mode)
```

## CLI usage

```sh
tokenburn                       # current month, all tools combined + per-tool breakdown
tokenburn --tool pi             # pi only   (--tool zed | all)
tokenburn --hourly              # today, bucketed by hour
tokenburn --daily               # this month, bucketed by day
tokenburn --monthly             # all logged history, bucketed by month
tokenburn --daily --all         # all logged history, bucketed by day
tokenburn --json                # machine-readable output
```

Flags are compatible with the original Nushell script (kept in `scripts/tokenburn.nu`).

## Web dashboard

```sh
just run-web                        # live dashboard, opens http://127.0.0.1:3000 in your browser
tokenburn-web --port 8080 --no-open # flags: --host, --port (or HOST / PORT env), --no-open
```

Binds `127.0.0.1` by default; `--host 0.0.0.0` exposes it on the network (there is no auth, so only do that deliberately).

The page is **live**: the filters are [Topcoat signals](https://github.com/tokio-rs/topcoat), changing one re-renders the dashboard without a page reload, and while the page is open the data refreshes itself every 5 s over a WebSocket (the footer shows `● live` and the last update time).

The live mode needs Topcoat's browser runtime bundle. Release archives, the DMG, the Windows installer and the `.deb`/`.rpm` already include it. When you build yourself, run `just run-web` (needs `cargo install topcoat-cli`), or run `topcoat asset bundle` and point `TOKENBURN_WEB_ASSETS` at the result. The bundle is searched in this order: `$TOKENBURN_WEB_ASSETS`, `<exe dir>/assets`, `<exe dir>/../share/tokenburn-web/assets`, `/usr/share/tokenburn-web/assets`, `/usr/local/share/tokenburn-web/assets`. Without a bundle `tokenburn-web` still works and serves a static dashboard (GET-form filters, reload every 10 s), which is what `cargo install tokenburn-web` gives you.

| URL | Description |
|-----|-------------|
| `/?tool=pi&bucket=daily&window=month` | dashboard (all params optional, invalid values fall back to defaults) |
| `/api/report?bucket=monthly` | same report as JSON |
| `/api/health` | liveness probe |

It binds to localhost by default — token/cost data is private, so don't expose it without auth.

## TUI keys

| Key | Action |
|-----|--------|
| `←/→`, `Tab`, `1-4` | total / hourly / daily / monthly |
| `t` | cycle tool (all → pi → zed) |
| `w` | cycle window (today → month → all) |
| `p` | toggle per-project / per-model view |
| `o` | options popup — checkboxes and sliders: auto-refresh + interval, bar chart, pie chart (braille), pie slices |
| `T`, `[`, `]` | theme picker / previous / next theme |
| `↑/↓`, `j/k`, `g/G` | scroll |
| `r` | refresh |
| `q`, `Esc` | quit |

## Building from source

```sh
just build            # whole workspace
just run-cli --daily  # CLI
just run-tui          # TUI
just run-gui          # GUI
just run-web          # web dashboard
```

## Development

```sh
just check-all   # fmt + clippy + tests + nu script tests
just coverage    # line coverage report (cargo-llvm-cov)
just bench       # refresh timing and memory: just bench month 20
just --list      # every task
```

### Demo GIFs

The recordings in [`examples/vhs/generated/`](examples/vhs/generated) are stored with **Git LFS** (`git lfs install` once, then a normal clone fetches them). They are generated from synthetic data (`examples/vhs/fixture.sh`) — never from your real logs:

```sh
just vhs-tui        # TUI + CLI tapes (examples/vhs/*.tape) — needs vhs, ttyd, ffmpeg
just screenshots    # GUI (macOS) + web screenshots and GIFs — needs ffmpeg, node, a Chromium browser
just vhs-tape filter
```

| Tape | Shows |
|------|-------|
| `dashboard` | report, charts, bucket / tool cycling, projects view |
| `filter` | `/` free-text filter, `-exclude`, `f` tool checkboxes |
| `themes` | theme picker and `[` / `]` cycling |
| `options` | panel toggles and the auto-refresh interval |
| `live-refresh` | the numbers updating by themselves |
| `cli` | table, buckets, `--json` |

## Architecture

```
tokenburn (cli) ──┐
tokenburn-tui ────┼──▶ tokenburn-core ──▶ rusqlite, zstd, serde_json, chrono
tokenburn-gui ────┤
tokenburn-web ────┘
```

```
crates/
├── tokenburn-core/   # collectors (pi, Zed, Claude, Codex, Copilot, Gemini, OpenCode, Amp), Query, aggregation, formatting
├── tokenburn-cli/    # clap args + table renderer
├── tokenburn-tui/    # app / events / layout / features / widgets
├── tokenburn-web/    # flat: app, pages, api, components, params, data, style
└── tokenburn-gui/    # state / message / update / view / features / widgets
```

## License

MIT — see [LICENSE](LICENSE).
