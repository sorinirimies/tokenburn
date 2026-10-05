# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]
### Features
- Rust workspace: `tokenburn-core`, clap CLI `tokenburn`, Ratatui `tokenburn-tui`, Iced `tokenburn-gui`
- Native pi + Zed collectors (rusqlite + zstd) — no `python3`/`zstd` binaries required
- `--json` output for the CLI
- `tokenburn-web`: Topcoat dashboard with signal-driven filters and live auto-refresh (WebSocket), static fallback, `/api/report` JSON
