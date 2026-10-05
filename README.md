<div align="center">

# 🔥 TokenBurn

**Local AI token & cost usage for pi and Zed — CLI, terminal UI, desktop GUI & web dashboard, all in Rust**

[![Crates.io](https://img.shields.io/crates/v/tokenburn.svg)](https://crates.io/crates/tokenburn)
[![docs.rs](https://docs.rs/tokenburn-core/badge.svg)](https://docs.rs/tokenburn-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

---

TokenBurn reads **local logs only** — no network calls, no external service:

| Source | Location | Granularity |
|--------|----------|-------------|
| [pi](https://github.com/earendil-works/pi-coding-agent) | `~/.pi/agent/sessions/**/*.jsonl` | per assistant turn: input / output / cache read / cache write tokens + estimated cost |
| Zed | `threads.db` (sqlite + zstd JSON) | per thread: input / output tokens only (Zed stores no `$` cost → always `0`) |

One Rust workspace, four front-ends sharing one core:

| Binary | Crate | Use case |
|--------|-------|----------|
| `tokenburn` | `tokenburn` | clap CLI — scripts, pipes, `--json` |
| `tokenburn-tui` | `tokenburn-tui` | Ratatui terminal UI — interactive report browser |
| `tokenburn-gui` | `tokenburn-gui` | Iced desktop GUI — pickers, themes, share bars |
| `tokenburn-web` | `tokenburn-web` | [Topcoat](https://github.com/tokio-rs/topcoat) web dashboard — server-rendered HTML + JSON API |

No `python3` or `zstd` binary needed (sqlite and zstd are bundled).

## Installation

```sh
cargo install tokenburn        # CLI
cargo install tokenburn-tui    # Terminal UI
cargo install tokenburn-gui    # Desktop GUI
cargo install tokenburn-web    # Web dashboard
```

## CLI usage

```sh
tokenburn                       # current month, pi+zed combined + per-tool breakdown
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

The live mode needs Topcoat's browser runtime bundle (`cargo install topcoat-cli`, then `just run-web`, or `topcoat asset bundle` and ship `assets/` beside the binary). Without a bundle `tokenburn-web` still works and serves a static dashboard (GET-form filters, reload every 60 s).

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
just --list      # every task
```

## Architecture

```
tokenburn (cli) ──┐
tokenburn-tui ────┼──▶ tokenburn-core ──▶ rusqlite, zstd, serde_json, chrono
tokenburn-gui ────┤
tokenburn-web ────┘
```

```
crates/
├── tokenburn-core/   # collectors (pi, zed), Query, aggregation, formatting
├── tokenburn-cli/    # clap args + table renderer
├── tokenburn-tui/    # app / events / layout / features / widgets
├── tokenburn-web/    # flat: app, pages, api, components, params, data, style
└── tokenburn-gui/    # state / message / update / view / features / widgets
```

## License

MIT — see [LICENSE](LICENSE).
