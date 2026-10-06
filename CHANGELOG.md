# Changelog

All notable changes to this project are documented here. It is generated from
[Conventional Commits](https://www.conventionalcommits.org) by
[git-cliff](https://git-cliff.org) — do not edit it by hand.

## [0.1.4](https://github.com/sorinirimies/tokenburn/compare/v0.1.3...v0.1.4) — 2026-10-06

### 📚 Documentation

- Changelog covers front-end and release commits and dependency updates; commit conventions ([`13e071e`](https://github.com/sorinirimies/tokenburn/commit/13e071ec7d0fc9818f5c63dc404cca41a8aa728d))
- Install badges for the TUI, GUI and web crates ([`830e909`](https://github.com/sorinirimies/tokenburn/commit/830e909c46767e8354dc914e8ae7855ef4faef40))
- Rewrite the README around the TUI, GUI and web front-ends; drop the performance section ([`07cb429`](https://github.com/sorinirimies/tokenburn/commit/07cb4293480800869062ced4c866a7dc848d6b8f))

### 🔄 CI and release

- **release:** Install the aarch64 libc headers on Gitea (apt skipped them under --no-install-recommends) ([`b20e460`](https://github.com/sorinirimies/tokenburn/commit/b20e460fc963763025a37265b0645cd311eae930))

## [0.1.3](https://github.com/sorinirimies/tokenburn/releases/tag/v0.1.3) — 2026-10-06

### ✨ Features

- Clickable, interactive, animated charts in the GUI and web (drill-down by bar/slice); no manual refresh anywhere ([`8c7f141`](https://github.com/sorinirimies/tokenburn/commit/8c7f141fdc0c40e817bc66872cb90de13401b4d3))
- TUI row filters and auto-update on every front-end ([`8bdfacb`](https://github.com/sorinirimies/tokenburn/commit/8bdfacb95667d621c812f54f9ff5cfd499d0f58b))
- 43 shared colour themes for the TUI, GUI and web dashboard ([`77eaf3a`](https://github.com/sorinirimies/tokenburn/commit/77eaf3a60b777d8cbd2356c567edae87e9f1d0bc))
- Collect usage from Claude Code, Codex, Copilot CLI, Gemini CLI, OpenCode and Amp ([`f17c193`](https://github.com/sorinirimies/tokenburn/commit/f17c193a9c8f33a1fbd54e4f6e5ab8720228517d))
- **tui:** Use tui-spinner, tui-piechart, tui-checkbox, tui-slider; release tooling for 0.1.0 ([`051685d`](https://github.com/sorinirimies/tokenburn/commit/051685dd6d1b62ef836f1db147404049270ba09e))
- Cross-platform release packaging, X11+Wayland GUI, per-OS log paths, web runtime bundling ([`644adec`](https://github.com/sorinirimies/tokenburn/commit/644adec47af16ae9529e9800cd3ee1152480f1cf))
- Tokenburn workspace — core, clap CLI, Ratatui TUI, Iced GUI and Topcoat web dashboard ([`5c1c7fc`](https://github.com/sorinirimies/tokenburn/commit/5c1c7fc900be6077c9c29c24fb325fb35f19b432))

### 🐛 Bug fixes

- **ci:** Nightly dependency update — install nu via setup-nu (install-action has no 'nushell' tool) ([`b34d214`](https://github.com/sorinirimies/tokenburn/commit/b34d2144777c6b4ddcd9fca442bc505e7f4f2581))
- **ci:** Set MSRV to 1.90 (the Linux dependency graph needs it: ordered-float 5.5, uuid 1.27) ([`605c4f7`](https://github.com/sorinirimies/tokenburn/commit/605c4f7eb6866bd9154122296da817d19af8b800))
- **tui:** Satisfy clippy field_reassign_with_default in options tests ([`5f93f63`](https://github.com/sorinirimies/tokenburn/commit/5f93f6321c2a06caa78851a018d4e8e416072023))

### ⚡ Performance

- Per-file parse caches + dirty-only TUI redraw; feat: charts in GUI and web; test: web e2e, CLI e2e, view tests ([`0616e6a`](https://github.com/sorinirimies/tokenburn/commit/0616e6a6f73e93a02d5ee42c9fc7e4a08972b4ac))

### 🖥️ Front-ends

- Remove the manual refresh key (r / F5); everything self-updates ([`ecb1b33`](https://github.com/sorinirimies/tokenburn/commit/ecb1b334b7e6dc965f0d0cd5d9fbb434d6f15716))
- Remove the Refresh button (everything auto-updates); refresh GUI screenshots ([`96756f2`](https://github.com/sorinirimies/tokenburn/commit/96756f202eaa8b2b3f8b56f1a2997406f83b3c2b))

### 📚 Documentation

- Rules for demo assets ([`f364319`](https://github.com/sorinirimies/tokenburn/commit/f3643199dfa24193ed2522ee081e39d4e6f98663))
- VHS tapes + GUI/web screenshots (git LFS), README charts/perf, just bench/coverage/screenshots ([`571e524`](https://github.com/sorinirimies/tokenburn/commit/571e524ea7a59ee60f7cd8a73d9f3a6da0a4e9e5))

### 📦 Build

- Slim debug profile (line tables only) so target/ stays small ([`bae86f7`](https://github.com/sorinirimies/tokenburn/commit/bae86f7c06aedb18538efcff6c896b7a449380cd))

### 🔄 CI and release

- Run the quality gate first; bump, changelog, commit and tag only once it is green, with rollback ([`2908f9a`](https://github.com/sorinirimies/tokenburn/commit/2908f9a773006b0f73ae3aa4a624ed965824bf13))
- Release only when something that ships changed (ignore dev/test-only lockfile churn) ([`560ee0f`](https://github.com/sorinirimies/tokenburn/commit/560ee0fbdb2a40d9621d2c424a91be20cbda7fcb))
- Drop the Linux GUI -dev apt packages (nothing links them) and retry apt ([`6575b50`](https://github.com/sorinirimies/tokenburn/commit/6575b508840ee92f8ba40c3ef7478d7f267aef0c))
- Independent fully-automatic nightly dependency updates on GitHub and Gitea (no mirroring) ([`27a232b`](https://github.com/sorinirimies/tokenburn/commit/27a232b5aafde71d925d1211de1e524ae092c856))


