<div align="center">

# 🔥 TokenBurn

**See what your AI coding agents cost you — in tokens and dollars, from the logs already on your disk.**

A terminal UI, a desktop GUI and a web dashboard (plus a CLI for scripts), all in Rust, all showing the same live numbers.

**Cross-platform: Linux, macOS and Windows are supported.**

[![Crates.io](https://img.shields.io/crates/v/tokenburn.svg)](https://crates.io/crates/tokenburn)
[![docs.rs](https://docs.rs/tokenburn-core/badge.svg)](https://docs.rs/tokenburn-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[![TUI installs](https://img.shields.io/crates/d/tokenburn-tui?label=tokenburn-tui%20installs&logo=rust&color=orange)](https://crates.io/crates/tokenburn-tui)
[![GUI installs](https://img.shields.io/crates/d/tokenburn-gui?label=tokenburn-gui%20installs&logo=rust&color=orange)](https://crates.io/crates/tokenburn-gui)
[![Web installs](https://img.shields.io/crates/d/tokenburn-web?label=tokenburn-web%20installs&logo=rust&color=orange)](https://crates.io/crates/tokenburn-web)
[![Audit](https://github.com/sorinirimies/tokenburn/actions/workflows/security.yml/badge.svg)](https://github.com/sorinirimies/tokenburn/actions/workflows/security.yml)
[![Fuzz](https://github.com/sorinirimies/tokenburn/actions/workflows/fuzz.yml/badge.svg)](https://github.com/sorinirimies/tokenburn/actions/workflows/fuzz.yml)

<img src="examples/vhs/generated/dashboard.gif" alt="TokenBurn terminal UI" width="900">

</div>

---

TokenBurn reads **local logs only** — no network calls, no account, no telemetry. It understands **pi, Zed, Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode, Amp, Cline, Roo Code, Kilo Code and Qwen Code**, shows where the tokens (and money) went, and keeps itself up to date while you work.

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
tokenburn --currency EUR   # costs in euros (see "Currencies" below)
tokenburn --json           # machine-readable (always US dollars)
```

![CLI](examples/vhs/generated/cli.gif)

## What it reads

| `--tool` | Tool | Where it reads (default) | `$` cost |
|----------|------|--------------------------|:---:|
| `pi` | [pi](https://github.com/earendil-works/pi-coding-agent) | `~/.pi/agent/sessions/**/*.jsonl` (`$TOKENBURN_PI_SESSIONS`, `$PI_CODING_AGENT_DIR`) | ✅ |
| `zed` | Zed agent panel | `threads.db` (sqlite + zstd JSON; `$TOKENBURN_ZED_DB`) | – |
| `claude` | Claude Code | `~/.claude/projects/**/*.jsonl` (`$CLAUDE_CONFIG_DIR`) | when logged |
| `codex` | OpenAI Codex CLI | `~/.codex/sessions/**/*.jsonl` (`$CODEX_HOME`) | – |
| `copilot` | GitHub Copilot CLI | `~/.copilot/session-state` (`$COPILOT_HOME`) | – |
| `gemini` | Google Gemini CLI | `~/.gemini/tmp/*/chats` (`$GEMINI_DATA_DIR`) | – |
| `opencode` | OpenCode | `~/.local/share/opencode/opencode.db` (`$OPENCODE_DATA_DIR`) | ✅ |
| `amp` | Amp | `~/.local/share/amp/threads/*.json` (`$AMP_DATA_DIR`) | – |
| `cline` | Cline | VS Code-family `globalStorage/saoudrizwan.claude-dev/tasks/*/ui_messages.json`, `~/.cline/data` (`$CLINE_DATA_DIR`) | ✅ |
| `roo` | Roo Code | `globalStorage/rooveterinaryinc.roo-cline/tasks/*/ui_messages.json` (`$ROO_DATA_DIR`) | ✅ |
| `kilo` | Kilo Code | `globalStorage/kilocode.kilo-code/tasks/*/ui_messages.json` (`$KILO_DATA_DIR`) | ✅ |
| `qwen` | Qwen Code | `~/.qwen/tmp/*/chats` (`$QWEN_DATA_DIR`) | – |

`all` (the default) reads every tool that is installed; one that isn't simply contributes nothing. Cost is shown only where the tool logs one — otherwise `$0.00` rather than a guess (token counts are exact). The Cline-family readers look in the `globalStorage` of VS Code, Insiders, VSCodium, Cursor, Windsurf and Positron. Cursor's own chat, Windsurf's Cascade, VS Code Copilot Chat and JetBrains AI keep no usable token log on disk, so they can't be supported.

> The Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode, Amp, Cline, Roo Code, Kilo Code and Qwen Code readers follow the formats documented by [ccusage](https://github.com/ryoppippi/ccusage) and are covered by fixture tests; pi and Zed were also checked against real logs (Cline, Roo, Kilo and Qwen only against fixtures so far). If one misreads your data, open an issue with a (redacted) log line.

## Shared by every front-end

**Always up to date.** Nothing to press — each front-end re-reads the logs itself, and only re-parses the files that changed, so leaving one open is cheap.

| | Refresh |
|---|---|
| TUI | every 5 s (5–300 s, or off, in the `o` popup) |
| GUI | every 10 s |
| Web | every 5 s over a WebSocket (every 10 s without the live runtime) |

![Auto-update: a new turn is logged and the numbers change by themselves](examples/vhs/generated/live-refresh.gif)

**Savings.** Next to what you spent, every front-end shows what you *saved* — in tokens and money, per source:

| Source | Where it comes from | Counted for |
|---|---|---|
| Prompt cache | `cache_read` tokens in the usage logs, credited at 90 % of the input price | every tool, follows the filter / drill-down |
| [rtk](https://github.com/rtk-ai/rtk) | tokens rtk filtered out of command output, from its `history.db` (override: `TOKENBURN_RTK_DB`) | `tool=all` only — rtk cannot say which agent ran a command |
| pi compaction | context removed by pi's compaction entries (size before vs. the next turn) | pi |

Money is an **estimate**: saved tokens × your input price, `input_price_per_mtok` in `settings.json` (default `$3.00` per million). Shown in the TUI header and, with the `m` metric on *saved*, as a panel under the charts, as a GUI card + panel, a web card + table (JSON at `/api/savings`) and in the CLI with `tokenburn --savings [--input-price 15]`. In the GUI and web, the **chart** dropdown (tokens / cost / **saved**) plots savings per hour, day or month with the same bucket and window filters; that chart covers prompt cache and pi compaction per tool, while rtk (not attributable to a tool) is in the totals and the panel.

**Metric.** The chart's *tokens / cost / saved* dropdown also picks what the headline cards show: token counts, spend (total, per turn, per 1M tokens) or savings (money, tokens, share, per source). The saved breakdown — and its per-interval list — appears only with *saved*, so it is out of the way by default.

**Intervals.** Savings follow the same buckets as everything else — **minutely, hourly, daily or monthly** (`--minutely`, `--hourly`, … / the *bucket* filter / TUI keys `2`–`5`): the saved panel lists what was saved in each interval, rtk and pi compaction land in the minute they happened, and clicking a bar narrows the totals to that interval. A selected project keeps only its own cache savings, since rtk and compactions are not attributed to projects.

**Charts.** Tokens or cost over time (one stacked bar per hour, day or month, one colour per tool) and a donut of the biggest projects. In the GUI and web dashboard they animate in, and clicking drills down:

| Click | Effect |
|-------|--------|
| a **bar** | table, totals and donut are scoped to that bucket |
| a **donut slice** (or its legend row) | table, totals and bars are scoped to that project |
| both | they intersect |
| the same one again, or **✕ clear** | back to everything |

Changing the tool, view or window drops the selection.

**Responsive.** All three front-ends adapt to the space they get, on the same three breakpoints — **wide** (≥ 980 px, ≥ 110 terminal columns), **medium** (≥ 700 px, ≥ 70 columns) and **narrow** — so a phone-sized browser, a half-screen window and an 80×24 terminal are all comfortable:

| | Wide | Medium | Narrow |
|---|---|---|---|
| **Layout** | everything: all columns, charts side by side | the cache columns drop out, charts stack | compact header and controls (they wrap), only the essential columns, whole page scrolls |
| **Web** | <img src="examples/vhs/generated/web-wide.png" width="360"> | <img src="examples/vhs/generated/web-medium.png" width="230"> | <img src="examples/vhs/generated/web-narrow.png" width="130"> |
| **GUI** | <img src="examples/vhs/generated/gui-wide.png" width="360"> | <img src="examples/vhs/generated/gui-medium.png" width="230"> | <img src="examples/vhs/generated/gui-narrow.png" width="130"> |

The terminal UI follows the terminal size live (it re-lays itself out as you resize):

| Medium (~95 columns) | Narrow (~65 columns) |
|---|---|
| <img src="examples/vhs/generated/medium.gif" width="440"> | <img src="examples/vhs/generated/narrow.gif" width="300"> |

- **TUI:** the cache columns go at < 110 columns; below 70 only `total` and `cost` stay, the pie and the theme chip make room (the currency stays), and the hints shrink. Charts shrink on short terminals and vanish below 22 rows so the table keeps the space; under 40×10 you get a "terminal too small" notice instead of a garbled screen.
- **GUI:** try it with `TOKENBURN_GUI_SIZE=400x820 tokenburn-gui` (the window can be as small as 320×420).
- **Web:** plain CSS media queries on the same breakpoints; tables scroll sideways instead of overflowing, and the chart text gets larger and sparser when it is scaled down.

**43 themes**, dark and light — Dracula, Nord, Tokyo Night, Catppuccin, Gruvbox, Solarized, Kanagawa, Rosé Pine, Everforest and more — shared by every front-end, including one colour per tool in the charts. Pick one with `--theme "Tokyo Night"` (`--list-themes` lists them), or in the UI; the choice is remembered in `settings.json` (`~/.config/tokenburn/`, `~/Library/Application Support/tokenburn/`, `%APPDATA%\tokenburn\`; override with `TOKENBURN_CONFIG_DIR`), so all front-ends start with the same theme.

![Theme picker](examples/vhs/generated/themes.gif)

**Currencies.** Every tool logs its cost in US dollars; pick another currency to see it converted — 41 are built in (EUR, GBP, CHF, JPY, RON, RUB, UAH, PLN, CZK, HUF, INR, CNY, BRL, CAD, AUD, AED, and more; `--list-currencies` prints them all). Choose one with `--currency EUR`, the pick list in the GUI, the select in the web dashboard (`?currency=EUR`) or `C` in the TUI; the choice is remembered in `settings.json` for every front-end. Amounts keep cents (`$0.04`), and yen, won, forint and the like have none.

TokenBurn never goes online, so the exchange rates are **fixed estimates** (marked `≈` next to the cost label). For exact ones, add your own to `settings.json` — they win over the built-in values and lose the `≈`:

```json
{ "currency": "EUR", "rates": { "EUR": 0.91, "RON": 4.55 } }
```

(rates are units per US dollar). JSON output (`--json`, `/api/report`) always stays in dollars.

## Install

### Prebuilt (GitHub releases)

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
| `←/→`, `Tab`, `1-5` | total / minutely / hourly / daily / monthly |
| `t` / `w` | cycle tool / window (today → month → all) |
| `m` | metric: tokens → cost → saved (headline figures, chart; saved adds the breakdown below the charts) |
| `/` | free-text filter: all words must match (tool, project, session id); `-word` excludes. `Enter` keeps, `Esc` clears |
| `f` | per-tool checkboxes with counts — `␣` toggle, `a` all, `n` none, `x` clear |
| `o` | options: auto-refresh and interval, bar chart, pie chart, pie slices |
| `T`, `[`, `]` | theme picker / previous / next |
| `C` | currency picker (preview as you move, saved) |
| `v` | jump to the saved metric: prompt cache, rtk, pi compaction |
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
just vhs-tape filter # one tape (narrow / medium: the responsive terminal demos)
```

## License

MIT — see [LICENSE](LICENSE).
