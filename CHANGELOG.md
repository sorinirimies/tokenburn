# Changelog

All notable changes to this project will be documented in this file.

## [0.1.3] - 2026-10-06
### ⚡ Performance
- per-file parse caches + dirty-only TUI redraw; feat: charts in GUI and web; test: web e2e, CLI e2e, view tests
### ✨ Features
- tokenburn workspace — core, clap CLI, Ratatui TUI, Iced GUI and Topcoat web dashboard
- cross-platform release packaging, X11+Wayland GUI, per-OS log paths, web runtime bundling
- use tui-spinner, tui-piechart, tui-checkbox, tui-slider; release tooling for 0.1.0
- collect usage from Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode and Amp
- 43 shared colour themes for the TUI, GUI and web dashboard
- TUI row filters and auto-update on every front-end
- clickable, interactive, animated charts in the GUI and web (drill-down by bar/slice); no manual refresh anywhere
### 🐛 Bug Fixes
- satisfy clippy field_reassign_with_default in options tests
- set MSRV to 1.90 (the Linux dependency graph needs it: ordered-float 5.5, uuid 1.27)
- nightly dependency update — install nu via setup-nu (install-action has no 'nushell' tool)
### 📚 Documentation
- VHS tapes + GUI/web screenshots (git LFS), README charts/perf, just bench/coverage/screenshots
- rules for demo assets
### 📦 Build
- slim debug profile (line tables only) so target/ stays small
### 🔄 CI
- independent fully-automatic nightly dependency updates on GitHub and Gitea (no mirroring)
- drop the Linux GUI -dev apt packages (nothing links them) and retry apt
- release only when something that ships changed (ignore dev/test-only lockfile churn)
