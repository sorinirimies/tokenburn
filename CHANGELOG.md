# Changelog

All notable changes to this project are documented here. It is generated from
[Conventional Commits](https://www.conventionalcommits.org) by
[git-cliff](https://git-cliff.org) — do not edit it by hand.

## [0.3.7](https://github.com/sorinirimies/tokenburn/compare/v0.3.5...v0.3.7) — 2026-10-31

### ✨ Features

- **tui:** Take theme colours from tui-nuances ([`f81cdbc`](https://github.com/sorinirimies/tokenburn/commit/f81cdbcf1590c01e129431967cb8fa7346ffa2b3))

### 📦 Dependencies

- Bump tokio and zstd to patch RUSTSEC advisories ([`4e55941`](https://github.com/sorinirimies/tokenburn/commit/4e55941cae1cd22fa66731391dc31ee04e76c955))
- Bump tui-nuances to 0.1.10 ([`ac80a81`](https://github.com/sorinirimies/tokenburn/commit/ac80a814a21d907a410f4cf0354bc03c5861bb6b))
- Nightly cargo update 2026-10-20 ([`39bfa71`](https://github.com/sorinirimies/tokenburn/commit/39bfa71fc1e8e5747fd2f00e10ed99605b67e859))

### 🔄 CI and release

- Nightly merges lockfile-only updates; bump + release only when a Cargo.toml requirement changes ([`32f189f`](https://github.com/sorinirimies/tokenburn/commit/32f189fa14417c940d5fa8134b09da013ceda18e))
- Nightly dependency update merges Cargo.lock only, no version bump or release ([`acd6cee`](https://github.com/sorinirimies/tokenburn/commit/acd6cee4bf9b16e7d2f82352b95d5ed5f9bce031))

## [0.3.5](https://github.com/sorinirimies/tokenburn/compare/v0.3.4...v0.3.5) — 2026-10-19

### 🔄 CI and release

- Retry rustup install on transient network failures ([`1d8f0b4`](https://github.com/sorinirimies/tokenburn/commit/1d8f0b467185f14219c0d773324477fbdc378713))

## [0.3.4](https://github.com/sorinirimies/tokenburn/compare/v0.3.3...v0.3.4) — 2026-10-19

### ✨ Features

- Caption the drop-downs; clearer web labels; update lockfile ([`ad119f0`](https://github.com/sorinirimies/tokenburn/commit/ad119f01447ca7efbdc4f1b7b0fe9b737bca240c))

### 🐛 Bug fixes

- **scripts:** Bump_version rejects versions equal to or older than the current one ([`87512c1`](https://github.com/sorinirimies/tokenburn/commit/87512c173013460b7ff812c48588f73daead162c))

### 🔄 CI and release

- Ignore unmaintained zbus stack advisories in cargo-deny ([`d993a60`](https://github.com/sorinirimies/tokenburn/commit/d993a60a7b03bb02b8dc2b841d1cb18c13fdcf96))

### 💄 Style

- Move button styles above test module (clippy) ([`a3ca74c`](https://github.com/sorinirimies/tokenburn/commit/a3ca74cbf18c0207cbd07c5ef9d020efb3f35582))
- Highlight action buttons in GUI and web ([`1730d2c`](https://github.com/sorinirimies/tokenburn/commit/1730d2cf5b3060a907d8bf72e2a0b3ece5ed6c31))

## [0.3.3](https://github.com/sorinirimies/tokenburn/compare/v0.3.2...v0.3.3) — 2026-10-18

### 📦 Dependencies

- Nightly cargo update 2026-10-18 ([`e09a30f`](https://github.com/sorinirimies/tokenburn/commit/e09a30fdd02e43576d1f94b7d6cdf14298aaa492))

## [0.3.2](https://github.com/sorinirimies/tokenburn/compare/v0.3.1...v0.3.2) — 2026-10-18

### ✨ Features

- Spending limits with desktop alerts; cost is always the last card/column ([`91d8c4a`](https://github.com/sorinirimies/tokenburn/commit/91d8c4aa6e13187eb563711364e1653cca00c60b))

## [0.3.1](https://github.com/sorinirimies/tokenburn/compare/v0.3.0...v0.3.1) — 2026-10-17

### 📦 Dependencies

- Nightly cargo update 2026-10-17 ([`ccbd6ed`](https://github.com/sorinirimies/tokenburn/commit/ccbd6edcf14216548bcf09f8118574d0ddf67022))

## [0.3.0](https://github.com/sorinirimies/tokenburn/compare/v0.2.7...v0.3.0) — 2026-10-17

### ✨ Features

- Show savings (prompt cache, rtk, pi compaction) in CLI, TUI, GUI and web ([`e87a01a`](https://github.com/sorinirimies/tokenburn/commit/e87a01a3e4975be5c085fac95ef9e296b0d9f729))

## [0.2.6](https://github.com/sorinirimies/tokenburn/compare/v0.2.5...v0.2.6) — 2026-10-16

### ✨ Features

- Add Cline, Roo Code, Kilo Code and Qwen Code providers; update README ([`37b5ef3`](https://github.com/sorinirimies/tokenburn/commit/37b5ef3ab4bc468da93f0c066705b65238389531))

### 🖥️ Front-ends

- Serve Cyberpunk CSS via /cyberpunk.css (inline style was HTML-escaped); fix flaky theme tests ([`1359fa8`](https://github.com/sorinirimies/tokenburn/commit/1359fa836a6745de502469e1c0126df50f714551))
- Exit on last page close and on Ctrl+C/SIGTERM ([`90b4414`](https://github.com/sorinirimies/tokenburn/commit/90b441481bfadf71e0c5e7ac86a40ee8ea22f521))

## [0.2.5](https://github.com/sorinirimies/tokenburn/compare/v0.2.4...v0.2.5) — 2026-10-15

### 📦 Dependencies

- Nightly cargo update 2026-10-15 ([`f32006e`](https://github.com/sorinirimies/tokenburn/commit/f32006e9da9229ce2c1b4aaadab7b36afa7f567c))

### 🔄 CI and release

- Prebuilt Nushell, cargo-deny + cargo-audit security workflow ([`f76633a`](https://github.com/sorinirimies/tokenburn/commit/f76633a843b59cbcb4b2989013bb8ed695677e0c))
- Prebuilt Nushell, cargo-deny + cargo-audit security workflow ([`fafcd35`](https://github.com/sorinirimies/tokenburn/commit/fafcd356fefac3525e5486b57859530c2c568825))

### 🔧 Chores

- **justfile:** Push/pull current branch instead of hardcoded main ([`ca8fca9`](https://github.com/sorinirimies/tokenburn/commit/ca8fca922eb3da06d5ee290cca6b2f919e4a2581))
- **justfile:** Skip unconfigured remotes in *-all recipes ([`cd1b706`](https://github.com/sorinirimies/tokenburn/commit/cd1b706cae06f3436d530e404cc116e285023f42))

## [0.2.4](https://github.com/sorinirimies/tokenburn/compare/v0.2.3...v0.2.4) — 2026-10-14

### 📦 Dependencies

- Nightly cargo update 2026-10-14 ([`7af4caa`](https://github.com/sorinirimies/tokenburn/commit/7af4caae79615c7d0f3d34d7a0db7ed711cc5620))

### 🔄 CI and release

- Split fuzz into own workflow; add audit & fuzz badges to README ([`972b2e9`](https://github.com/sorinirimies/tokenburn/commit/972b2e9ccce12e1191df27f9003e58686da09734))
- Add cargo-fuzz targets + fuzz smoke job; fix audit/fuzz findings ([`d858477`](https://github.com/sorinirimies/tokenburn/commit/d85847748ac2b1618d0fc33d485e2e48329e9225))
- Download rustup script before running ([`f1474bb`](https://github.com/sorinirimies/tokenburn/commit/f1474bbe90adc052875f772704eec77760510d19))

## [0.2.3](https://github.com/sorinirimies/tokenburn/compare/v0.2.2...v0.2.3) — 2026-10-13

### 📦 Dependencies

- Nightly cargo update 2026-10-13 ([`26a68cc`](https://github.com/sorinirimies/tokenburn/commit/26a68cc0117d076d58eceee606962386da8ff898))

## [0.2.2](https://github.com/sorinirimies/tokenburn/compare/v0.2.1...v0.2.2) — 2026-10-12

### ✨ Features

- Version label in titles, About and Settings tabs, theme selector last ([`078ac56`](https://github.com/sorinirimies/tokenburn/commit/078ac56e4224536f450d75ec60bd6f86bee8f700))

### 🔄 CI and release

- **security:** Add cargo-audit workflow and just audit recipe ([`07dae40`](https://github.com/sorinirimies/tokenburn/commit/07dae400eb86018b0126bb8504a3692c814dae8b))

## [0.2.1](https://github.com/sorinirimies/tokenburn/compare/v0.2.0...v0.2.1) — 2026-10-09

### 🐛 Bug fixes

- **gui:** Unresolved rustdoc link to macos ([`fcd841f`](https://github.com/sorinirimies/tokenburn/commit/fcd841f45dcd6833e569cc05edaef67039bc099e))

## [0.2.0](https://github.com/sorinirimies/tokenburn/compare/v0.1.9...v0.2.0) — 2026-10-09

### 🐛 Bug fixes

- **gui:** Add the macOS Window menu so Zoom, Move & Resize and the tiling shortcuts work ([`4f66056`](https://github.com/sorinirimies/tokenburn/commit/4f660565ed5aca8b7d7f09cdab708bbbaef06c67))

### ♻️ Refactor

- **gui:** Move the macOS window menu into a window module (window::setup) ([`f83ab3b`](https://github.com/sorinirimies/tokenburn/commit/f83ab3b8c94c52d9a67ebb226115002ae39babb4))

## [0.1.9](https://github.com/sorinirimies/tokenburn/compare/v0.1.8...v0.1.9) — 2026-10-07

### ⚡ Performance

- Lower memory — borrow rows instead of copying them (drill, TUI), stream huge Zed threads; add soak tests ([`7fee2a2`](https://github.com/sorinirimies/tokenburn/commit/7fee2a220747fc0732d55c2d543336cdd1923289))

### 🔄 CI and release

- **gitea:** Retry rustup and cargo downloads (a network timeout failed a run) ([`4fb3b20`](https://github.com/sorinirimies/tokenburn/commit/4fb3b206296e76ff19499321290d688f88f46c10))
- Release only for real upgrades; never for downgrades, checksum-only or dev-only changes ([`f7f0516`](https://github.com/sorinirimies/tokenburn/commit/f7f0516e0c8e2f5b5bf9c77e5e64a09ef2d5178e))

## [0.1.8](https://github.com/sorinirimies/tokenburn/compare/v0.1.7...v0.1.8) — 2026-10-07

### 📦 Dependencies

- Nightly cargo update 2026-10-07 ([`9fcec07`](https://github.com/sorinirimies/tokenburn/commit/9fcec07125e212e136c333989fc45d6c13002aa4))

## [0.1.7](https://github.com/sorinirimies/tokenburn/compare/v0.1.6...v0.1.7) — 2026-10-06

### 🔄 CI and release

- **release:** Prune old Gitea release files with nushell; no python in the release pipeline ([`b071dc8`](https://github.com/sorinirimies/tokenburn/commit/b071dc81bd2292838e40481773ffba257e8066f6))
- **release:** Keep only the latest release's files on Gitea (prune older assets) ([`10ca942`](https://github.com/sorinirimies/tokenburn/commit/10ca94230b0489f51bb3037ab10783b34adcb199))

### 🔧 Chores

- Ignore __pycache__ ([`9ff506c`](https://github.com/sorinirimies/tokenburn/commit/9ff506c8027a946a0b525e739cb6619b00f086de))

## [0.1.6](https://github.com/sorinirimies/tokenburn/compare/v0.1.5...v0.1.6) — 2026-10-06

### 📚 Documentation

- Prebuilt downloads are on GitHub only (Gitea is a private mirror) ([`0358907`](https://github.com/sorinirimies/tokenburn/commit/0358907142167321d70144f8ba8a394ace59aab2))

## [0.1.5](https://github.com/sorinirimies/tokenburn/compare/v0.1.4...v0.1.5) — 2026-10-06

### ✨ Features

- Responsive TUI, GUI and web on shared wide/medium/narrow breakpoints ([`22345d9`](https://github.com/sorinirimies/tokenburn/commit/22345d9e3ecadb136d3e901d43708544ca0d233f))
- Currency selector in the CLI, TUI, GUI and web (41 currencies incl. RON, RUB; costs in cents; user-set rates in settings.json) ([`eb972ca`](https://github.com/sorinirimies/tokenburn/commit/eb972ca7dc7ffd21d9283cbb80f7649325894203))
- Show costs with one decimal; share all number formatting in core::utils::fmt ([`7e4af5f`](https://github.com/sorinirimies/tokenburn/commit/7e4af5f49ebc9d25490af689c0af4e5b0b9d72a7))

### 📚 Documentation

- Say cross-platform up front; drop the platform support section ([`f3fc678`](https://github.com/sorinirimies/tokenburn/commit/f3fc678b7fcf3cce4d78e65f01ed74b64add288b))

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


