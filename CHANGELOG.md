# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]
### Features
- Rust workspace: `tokenburn-core`, clap CLI `tokenburn`, Ratatui `tokenburn-tui`, Iced `tokenburn-gui`
- Collectors for pi, Zed, Claude Code, Codex CLI, Copilot CLI, Gemini CLI, OpenCode and Amp (read in parallel, de-duplicated, cache-aware)
- Native pi + Zed collectors (rusqlite + zstd) — no `python3`/`zstd` binaries required
- `--json` output for the CLI
- TUI: share pie chart (`tui-piechart`), options popup with checkboxes/sliders (`tui-checkbox`, `tui-slider`), animated loading spinner (`tui-spinner`), auto-refresh
- Cross-platform: macOS, Linux (X11 + Wayland from one binary, glibc and static musl builds) and Windows
- Log locations resolved per platform with overrides (`TOKENBURN_PI_SESSIONS`, `TOKENBURN_ZED_DB`, pi's `PI_CODING_AGENT_*`)
- App icon set (PNG/ICO/ICNS/SVG), window icon, `.desktop` + AppStream metadata, Wayland `app_id`
- Packaging for every app: `.deb`/`.rpm`, AppImage, universal DMG, NSIS installer, portable archives with checksums
- Web runtime bundle shipped with every package (`TOKENBURN_WEB_ASSETS` search path); static fallback otherwise
- `tokenburn-web`: Topcoat dashboard with signal-driven filters and live auto-refresh (WebSocket), static fallback, `/api/report` JSON
