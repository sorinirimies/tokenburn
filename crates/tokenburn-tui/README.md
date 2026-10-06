# tokenburn-tui

Interactive terminal UI ([Ratatui](https://ratatui.rs)) for [TokenBurn](https://github.com/sorinirimies/tokenburn) — local AI token/cost usage for your AI coding agents (**pi**, **Zed**, **Claude Code**, **Codex**, **Copilot CLI**, **Gemini CLI**, **OpenCode**, **Amp**).

| Key | Action |
|-----|--------|
| `←/→`, `Tab`, `1-4` | total / hourly / daily / monthly |
| `t` | cycle tool (all → pi → zed) |
| `w` | cycle window (today → month → all) |
| `p` | toggle per-project / per-model view |
| `↑/↓`, `j/k` | scroll |
| `/` | filter rows by text (`word`, `-exclude`; Enter keep · Esc clear) |
| `f` | filter by tool (checkboxes with counts) |
| `o` | options popup |
| `T` | theme picker (43 themes, live preview) |
| `[` `]` | previous / next theme |
| `q`, `Esc` | quit |

## Built with

Besides [Ratatui](https://ratatui.rs): [`tui-spinner`](https://crates.io/crates/tui-spinner) (loading indicator), [`tui-piechart`](https://crates.io/crates/tui-piechart) (cost / token share by project, braille high-resolution), [`tui-checkbox`](https://crates.io/crates/tui-checkbox) and [`tui-slider`](https://crates.io/crates/tui-slider) (the `o` options popup: auto-refresh, refresh interval, chart toggles, pie slice count).

## Platforms

Works in any terminal on macOS, Linux and Windows (Windows Terminal, PowerShell, cmd). The static `x86_64-unknown-linux-musl` build runs on any Linux distro. Start options: `tokenburn-tui --daily --tool pi`.
