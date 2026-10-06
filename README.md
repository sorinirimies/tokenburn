<div align="center">

# 🔥 TokenBurn

**See what your AI coding agents cost you — in tokens and dollars, from the logs already on your disk.**

A terminal UI, a desktop GUI and a web dashboard (plus a CLI for scripts), all in Rust, all showing the same live numbers.

[![Crates.io](https://img.shields.io/crates/v/tokenburn.svg)](https://crates.io/crates/tokenburn)
[![docs.rs](https://docs.rs/tokenburn-core/badge.svg)](https://docs.rs/tokenburn-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[![TUI installs](https://img.shields.io/crates/d/tokenburn-tui?label=tokenburn-tui%20installs&logo=rust&color=orange)](https://crates.io/crates/tokenburn-tui)
[![GUI installs](https://img.shields.io/crates/d/tokenburn-gui?label=tokenburn-gui%20installs&logo=rust&color=orange)](https://crates.io/crates/tokenburn-gui)
[![Web installs](https://img.shields.io/crates/d/tokenburn-web?label=tokenburn-web%20installs&logo=rust&color=orange)](https://crates.io/crates/tokenburn-web)

<img src="examples/vhs/generated/dashboard.gif" alt="TokenBurn terminal UI" width="900">

</div>

---

TokenBurn reads **local logs only** — no network calls, no account, no telemetry. It understands **pi, Zed, Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode and Amp**, shows where the tokens (and money) went, and keeps itself up to date while you work.

## Choose your front-end

Same data, same charts, same 43 themes — pick whichever fits how you work.

### 🖥️ Terminal UI — `tokenburn-tui`

For when you live in the terminal. Browse the report, watch the charts, and filter as you type.

![TUI: report, token chart and share pie](examples/vhs/generated/dashboard.gif)

- Report table with a stacked token chart and a share-by-project pie
- **Instant filtering**: `/` free text (`claude -zed`), `f` per-tool checkboxes with row counts
- Options popup (`o`): toggle panels, auto-refresh interval (5–300 s or off)
- Theme picker with live preview (`T`), or `[` / `]` to cycle

```sh
cargo install tokenburn-tui && tokenburn-tui --daily
```

### 🪟 Desktop GUI — `tokenburn-gui`

A native window (Iced) with **interactive, animated charts**. Click a bar to scope everything to that day, click a slice to scope it to that project.

![GUI themes](examples/vhs/generated/gui.gif)

- Stacked bars over time (tokens or cost) and a project donut; hover for tooltips
- **Click to drill down** — the table, totals and the other chart follow
- macOS app, Windows installer, and Linux (one binary for **X11 and Wayland**)
- Drop-down pickers for tool, view, window and theme

```sh
cargo install tokenburn-gui && tokenburn-gui
```

### 🌐 Web dashboard — `tokenburn-web`

A local web page in your browser. Live over a WebSocket, drill-down by clicking, and every view is a shareable URL.

![Web: hover and click a bar, a slice and a legend row, then clear](examples/vhs/generated/web-interactive.gif)

- Server-rendered SVG charts with CSS animation — no JavaScript charting library
- Filters are [Topcoat](https://github.com/tokio-rs/topcoat) signals: changing one updates the page in place
- A drilled-down view is part of the URL: `?sel_bucket=2026-09-22&sel_project=pi%3Aatlas`
- JSON API at `/api/report`; binds to `127.0.0.1` by default

```sh
tokenburn-web            # opens http://127.0.0.1:3000
```

### ⌨️ CLI — `tokenburn`

For scripts and pipes.

```sh
tokenburn                  # this month, every tool, with a per-tool breakdown
tokenburn --daily --all    # all history, one line per day and tool
tokenburn --tool claude --monthly
tokenburn --json           # machine-readable
```

![CLI](examples/vhs/generated/cli.gif)

## What it reads

| `--tool` | Tool | Where it reads (default) | `$` cost |
|----------|------|--------------------------|:---:|
| `pi` | [pi](https://github.com/earendil-works/pi-coding-agent) | `~/.pi/agent/sessions/**/*.jsonl` | ✅ |
| `zed` | Zed agent panel | `threads.db` (sqlite + zstd JSON) | – |
| `claude` | Claude Code | `~/.claude/projects/**/*.jsonl` (`$CLAUDE_CONFIG_DIR`) | when logged |
| `codex` | OpenAI Codex CLI | `~/.codex/sessions/**/*.jsonl` (`$CODEX_HOME`) | – |
| `copilot` | GitHub Copilot CLI | `~/.copilot/session-state` (`$COPILOT_HOME`) | – |
| `gemini` | Google Gemini CLI | `~/.gemini/tmp/*/chats` (`$GEMINI_DATA_DIR`) | – |
| `opencode` | OpenCode | `~/.local/share/opencode/opencode.db` (`$OPENCODE_DATA_DIR`) | ✅ |
| `amp` | Amp | `~/.local/share/amp/threads/*.json` (`$AMP_DATA_DIR`) | – |

`all` (the default) reads every tool that is installed; one that isn't simply contributes nothing. Cost is shown only where the tool logs one — otherwise `$0.0000` rather than a guess (token counts are exact). Cursor, Windsurf, VS Code Copilot Chat and JetBrains AI keep no usable token log on disk, so they can't be supported.

> The Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode and Amp readers follow the formats documented by [ccusage](https://github.com/ryoppippi/ccusage) and are covered by fixture tests; pi and Zed were also checked against real logs. If one misreads your data, open an issue with a (redacted) log line.

Per-platform locations and the full list of overrides are in [Platform support](#platform-support).

## Shared by every front-end

**Always up to date.** Nothing to press — each front-end re-reads the logs itself, and only re-parses the files that changed, so leaving one open is cheap.

| | Refresh |
|---|---|
| TUI | every 5 s (5–300 s, or off, in the `o` popup) |
| GUI | every 10 s |
| Web | every 5 s over a WebSocket (every 10 s without the live runtime) |

![Auto-update: a new turn is logged and the numbers change by themselves](examples/vhs/generated/live-refresh.gif)

**Charts.** Tokens or cost over time (one stacked bar per hour, day or month, one colour per tool) and a donut of the biggest projects. In the GUI and web dashboard they animate in, and clicking drills down:

| Click | Effect |
|-------|--------|
| a **bar** | table, totals and donut are scoped to that bucket |
| a **donut slice** (or its legend row) | table, totals and bars are scoped to that project |
| both | they intersect |
| the same one again, or **✕ clear** | back to everything |

Changing the tool, view or window drops the selection.

**43 themes**, dark and light — Dracula, Nord, Tokyo Night, Catppuccin, Gruvbox, Solarized, Kanagawa, Rosé Pine, Everforest and more — shared by every front-end, including one colour per tool in the charts. Pick one with `--theme "Tokyo Night"` (`--list-themes` lists them), or in the UI; the choice is remembered in `settings.json` (`~/.config/tokenburn/`, `~/Library/Application Support/tokenburn/`, `%APPDATA%\tokenburn\`; override with `TOKENBURN_CONFIG_DIR`), so all front-ends start with the same theme.

![Theme picker](examples/vhs/generated/themes.gif)

## Install

### Prebuilt (GitHub / Gitea releases)

| Platform | Artifacts |
|----------|-----------|
| macOS | `tokenburn-<v>-macos.dmg` — `TokenBurn.app` + `Command Line Tools/` (CLI, TUI, web) |
| Windows | `tokenburn-<v>-windows-x86_64-setup.exe` (installer, optional PATH entry) or the `.zip` |
| Linux | `.deb` / `.rpm` per app, `tokenburn-{tui,gui}-<v>-<arch>.AppImage`, portable `.tar.gz` (glibc x86_64 / aarch64, static musl x86_64) |

Every portable archive holds all binaries for that target, the web runtime in `assets/` (so `tokenburn-web` runs live), the README and licence, plus a `.sha256`.

macOS: the app is ad-hoc signed, not notarised — right-click → Open the first time, or `xattr -dr com.apple.quarantine TokenBurn.app`.

### From crates.io

```sh
cargo install tokenburn        # CLI
cargo install tokenburn-tui    # terminal UI
cargo install tokenburn-gui    # desktop GUI
cargo install tokenburn-web    # web dashboard (static mode; see "Live mode" below)
```

No `python3` or `zstd` binary needed — sqlite and zstd are bundled.

## Using it

### TUI keys

| Key | Action |
|-----|--------|
| `←/→`, `Tab`, `1-4` | total / hourly / daily / monthly |
| `t` / `w` | cycle tool / window (today → month → all) |
| `p` | per-project / per-model view |
| `/` | free-text filter: all words must match (tool, project, session id); `-word` excludes. `Enter` keeps, `Esc` clears |
| `f` | per-tool checkboxes with counts — `␣` toggle, `a` all, `n` none, `x` clear |
| `o` | options: auto-refresh and interval, bar chart, pie chart, pie slices |
| `T`, `[`, `]` | theme picker / previous / next |
| `↑/↓`, `j/k`, `g/G` | scroll |
| `q`, `Esc` | quit (`Esc` first clears an active filter) |

Filters apply instantly to the rows already loaded; the header shows what is active, e.g. `⌕ filter:"claude"  tools:3/8  412/3,029 rows`.

![Filtering](examples/vhs/generated/filter.gif)

### Web dashboard

```sh
just run-web                        # live dashboard, opens http://127.0.0.1:3000
tokenburn-web --port 8080 --no-open # flags: --host, --port (or HOST / PORT), --no-open, --theme
```

| URL | Description |
|-----|-------------|
| `/?tool=pi&bucket=daily&window=month&metric=cost` | the dashboard (every parameter optional) |
| `/api/report?bucket=monthly` | the same report as JSON |
| `/api/themes`, `/api/health` | theme palettes, liveness probe |

It binds to `127.0.0.1`. `--host 0.0.0.0` exposes it on the network — there is no authentication, and token/cost data is private, so only do that deliberately.

**Live mode** needs Topcoat's browser runtime. Release archives, the DMG, the Windows installer and the `.deb`/`.rpm` include it. When building yourself, `just run-web` bundles it (needs `cargo install topcoat-cli`), or run `topcoat asset bundle` and point `TOKENBURN_WEB_ASSETS` at the result. Without it, `tokenburn-web` still works and serves a static dashboard (form filters, links instead of clicks, reload every 10 s) — which is what `cargo install tokenburn-web` gives you. Search order: `$TOKENBURN_WEB_ASSETS`, `<exe dir>/assets`, `<exe dir>/../share/tokenburn-web/assets`, `/usr/share/…`, `/usr/local/share/…`.

## Platform support

macOS, Linux and Windows, all four binaries:

| Binary | macOS (arm64 + x86_64) | Linux (x86_64 + aarch64) | Windows (x86_64) |
|--------|:---:|:---:|:---:|
| `tokenburn`, `tokenburn-tui`, `tokenburn-web` | ✅ | ✅ | ✅ |
| `tokenburn-gui` | ✅ `.app` in a universal DMG | ✅ X11 **and** Wayland | ✅ |

Log locations are found per platform; every one can be overridden:

| Source | macOS | Linux | Windows | Override |
|--------|-------|-------|---------|----------|
| pi | `~/.pi/agent/sessions` | `~/.pi/agent/sessions` | `%USERPROFILE%\.pi\agent\sessions` | `TOKENBURN_PI_SESSIONS`, `PI_CODING_AGENT_SESSION_DIR`, `PI_CODING_AGENT_DIR` |
| Zed | `~/Library/Application Support/Zed/threads/threads.db` | `$XDG_DATA_HOME/zed/…`, Flatpak `~/.var/app/dev.zed.Zed/…` | `%LOCALAPPDATA%\Zed\threads\threads.db` | `TOKENBURN_ZED_DB` |

### Linux notes

- **One GUI binary serves X11 and Wayland.** It links only libc/libm and loads X11, Wayland, xkbcommon and the GPU libraries at runtime. It picks Wayland when `WAYLAND_DISPLAY` is set; force X11 with `env -u WAYLAND_DISPLAY tokenburn-gui`.
- **Software rendering** is the automatic fallback without a usable GPU (VMs, remote desktops); force it with `ICED_BACKEND=tiny-skia`.
- **glibc builds** (AppImage, `.deb`, `.rpm`, `*-gnu`) are linked on Ubuntu 22.04, so they need glibc ≥ 2.35. **Static musl builds** (CLI, TUI, web) run anywhere — Alpine, containers, NixOS.
- The GUI needs `libxkbcommon` and fonts, as any desktop Linux has.

## Development

```sh
just build              # whole workspace
just run-tui            # run-cli / run-tui / run-gui / run-web
just check-all          # fmt + clippy + tests + nu script tests
just coverage           # line coverage (cargo-llvm-cov)
just --list             # everything else
```

```
tokenburn (cli) ──┐
tokenburn-tui ────┼──▶ tokenburn-core ──▶ rusqlite, zstd, serde_json, chrono
tokenburn-gui ────┤
tokenburn-web ────┘
```

| Crate | |
|-------|---|
| `tokenburn-core` | the collectors, queries, aggregation, chart data and drill-down logic, themes, settings |
| `tokenburn-cli` | clap args and the table renderer |
| `tokenburn-tui` | Ratatui: app / events / layout / features / widgets |
| `tokenburn-gui` | Iced: state / message / update / view / features / widgets |
| `tokenburn-web` | Topcoat: app / pages / api / components / charts / params / data / style |

### Demo recordings

The GIFs above live in [`examples/vhs/generated/`](examples/vhs/generated) and are stored with **Git LFS** (`git lfs install` once). They are generated from synthetic data (`examples/vhs/fixture.sh`), never from real logs:

```sh
just vhs-tui         # TUI + CLI tapes (examples/vhs/*.tape) — needs vhs, ttyd, ffmpeg
just screenshots     # GUI (macOS) + web screenshots and GIFs — needs ffmpeg, node, a Chromium browser
just vhs-tape filter # one tape
```

## License

MIT — see [LICENSE](LICENSE).
