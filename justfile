# tokenburn workspace — task runner
# Install just:      cargo install just
# Install vhs:       brew install vhs  OR  go install github.com/charmbracelet/vhs@latest
# Install git-cliff: cargo install git-cliff
# Usage: just <task>
# ── Default ───────────────────────────────────────────────────────────────────

default:
    @just --list

# ── Tool checks ───────────────────────────────────────────────────────────────

_check-git-cliff:
    @command -v git-cliff >/dev/null 2>&1 || { \
        echo "❌ git-cliff not found. Install with: cargo install git-cliff"; exit 1; \
    }

# Check nu (nushell) is available
_check-nu:
    @command -v nu >/dev/null 2>&1 || { \
        echo "❌ nu (nushell) not found. Install: https://www.nushell.sh"; exit 1; \
    }

_check-vhs:
    @command -v vhs >/dev/null 2>&1 || { \
        echo "❌ vhs not found."; \
        echo "   macOS:      brew install vhs"; \
        echo "   Any:        go install github.com/charmbracelet/vhs@latest"; \
        exit 1; \
    }

# Install all recommended development tools
install-tools:
    @echo "Installing development tools…"
    @command -v git-cliff >/dev/null 2>&1 || cargo install git-cliff --locked
    @command -v nu >/dev/null 2>&1 || cargo install nu --locked
    @echo "✅ All tools installed!"

# ── Build ─────────────────────────────────────────────────────────────────────

# Build the entire workspace (dev)
build:
    cargo build --workspace

# Build only the core library (dev)
build-core:
    cargo build -p tokenburn-core

# Build only the CLI crate (dev)
build-cli:
    cargo build -p tokenburn

# Build only the web crate (dev)
build-web:
    cargo build -p tokenburn-web

# Build only the GUI crate (dev)
build-gui:
    cargo build -p tokenburn-gui

# Build only the TUI crate (dev)
build-tui:
    cargo build -p tokenburn-tui

# Build release binaries for CLI, GUI, web and TUI
build-release:
    cargo build --release -p tokenburn
    cargo build --release -p tokenburn-gui
    cargo build --release -p tokenburn-web
    cargo build --release -p tokenburn-tui

# ── Run ───────────────────────────────────────────────────────────────────────

# Run the clap CLI (pass args after `--`, e.g. `just run-cli --daily`)
run-cli *args:
    cargo run -p tokenburn -- {{ args }}

# Check topcoat-cli is available
_check-topcoat:
    @command -v topcoat >/dev/null 2>&1 || { \
        echo "❌ topcoat not found. Install: cargo install topcoat-cli"; exit 1; \
    }

# Bundle the Topcoat browser runtime next to the debug binary (needed for the live dashboard)
bundle-web: _check-topcoat
    topcoat asset bundle --package tokenburn-web --out target/debug/assets

# Serve the live web dashboard (http://127.0.0.1:3000; override with PORT/HOST)
run-web: bundle-web
    cargo run -p tokenburn-web

# Serve the web dashboard with automatic rebuilds (topcoat dev server)
dev-web: _check-topcoat
    cd crates/tokenburn-web && topcoat dev

# Serve the static (no-runtime) web dashboard — works without topcoat-cli
run-web-static:
    cargo run -p tokenburn-web

# Launch the Iced desktop GUI
run-gui:
    cargo run -p tokenburn-gui

# Launch the Ratatui terminal UI
run-tui:
    cargo run -p tokenburn-tui

# Alias: default run launches the CLI report
run: run-cli

# ── Test ──────────────────────────────────────────────────────────────────────

# Run the full workspace test suite
test:
    cargo test --workspace --locked --all-features --all-targets

# Test only the core library
test-core:
    cargo test -p tokenburn-core --all-features

# Test only the CLI crate
test-cli:
    cargo test -p tokenburn --all-features

# Test only the web crate
test-web:
    cargo test -p tokenburn-web --all-features

# Test only the GUI crate
test-gui:
    cargo test -p tokenburn-gui --all-features

# Test only the TUI crate
test-tui:
    cargo test -p tokenburn-tui --all-features

# Run Nu script tests
test-nu: _check-nu
    nu scripts/tests/run_all.nu

# Run both Rust and Nu tests
test-all-nu: test test-nu
    @echo "✅ All Rust and Nu tests passed!"

# ── Code quality ──────────────────────────────────────────────────────────────

# Check without building
check:
    cargo check --workspace

# Format all code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run clippy across the workspace
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated

# Run all quality checks (format, clippy, test, nu) — must pass before a release.

# Auto-formats first, then verifies no changes remain (catches unstaged format diffs).
check-all: fmt clippy test test-nu
    @echo "🔍 Verifying formatting is clean…"
    cargo fmt --all -- --check
    @echo "✅ All checks passed!"

# Full pre-release quality gate — everything in check-all plus a release build.
check-release: check-all build-release
    @echo "✅ Release quality gate passed (fmt + clippy + test + nu + release build)!"

# ── VHS Demo GIFs ─────────────────────────────────────────────────────────────

VHS := "examples/vhs"
VHS_GENERATED := "examples/vhs/generated"

# Record every terminal demo GIF (TUI + CLI), then the GUI/web screenshots + GIFs
vhs-all: vhs-tui screenshots

# Record the terminal demos (TUI + CLI) with VHS, on the synthetic fixture data
vhs-tui: _check-vhs
    #!/usr/bin/env sh
    set -e
    mkdir -p {{ VHS_GENERATED }}
    echo "╔════════════════════════════════════════════╗"
    echo "║   Tapes (TUI + CLI, VHS)                  ║"
    echo "╚════════════════════════════════════════════╝"
    for tape in {{ VHS }}/*.tape; do
        [ -f "$tape" ] || continue
        echo "▶  $tape"
        vhs "$tape" || echo "❌ Failed: $tape"
    done
    echo "✅ Demos done → {{ VHS_GENERATED }}/"

# Render a single tape by name (e.g. just vhs-tape filter)
vhs-tape name: _check-vhs
    #!/usr/bin/env sh
    if [ -f "{{ VHS }}/{{ name }}.tape" ]; then
        vhs "{{ VHS }}/{{ name }}.tape" && echo "✅ Done."
    else
        echo "❌ Tape not found: {{ name }}.tape"
        just vhs-list
        exit 1
    fi

# List all VHS tapes
vhs-list:
    @ls {{ VHS }}/*.tape | sed 's|.*/||; s|\.tape||' | sed 's/^/  /'

# GUI + web screenshots and GIFs (VHS can't record a window or a browser; GUI half is macOS-only)
screenshots: _check-topcoat
    cargo build -p tokenburn-gui -p tokenburn-web
    topcoat asset bundle --package tokenburn-web --out target/debug/assets
    bash examples/capture/all.sh

# Timing and memory of a refresh: just bench [month|all|today] [runs]
bench window="all" runs="10":
    cargo run --release -p tokenburn-core --example bench -- {{ window }} {{ runs }}

# Memory soak tests: repeat the refresh path of the core, TUI and GUI state and print the
# resident memory; a leak shows as a steady climb (just soak 200)
soak rounds="100":
    cargo run --release -p tokenburn-core --example soak -- {{ rounds }} all
    cargo run --release -p tokenburn-tui --example soak -- {{ rounds }}
    cargo run --release -p tokenburn-gui --example soak -- {{ rounds }}

# Line coverage (needs cargo-llvm-cov); writes target/llvm-cov/html
coverage:
    cargo llvm-cov --workspace --html
    @echo "→ target/llvm-cov/html/index.html"

# ── Packaging ─────────────────────────────────────────────────────────────────
# Release artifacts are normally built by CI (GitHub / Gitea release workflows).
# These recipes reproduce them locally. Every package recipe builds with an
# explicit --target so the output lands in target/<triple>/release, exactly
# where the packaging scripts look.

LINUX_TARGET := "x86_64-unknown-linux-gnu"
MUSL_TARGET := "x86_64-unknown-linux-musl"

# Regenerate every icon asset (PNG set, .ico, .icns, window icon) — needs Pillow
icons:
    python3 scripts/gen_icons.py

# Bundle the web runtime into dist/web-assets + target/release/assets (live mode for local release runs)
bundle-web-release: _check-topcoat
    bash scripts/ci/bundle_web.sh dist/web-assets
    mkdir -p target/release && rm -rf target/release/assets && cp -R dist/web-assets target/release/assets

# Build all four apps for one target (GUI skipped for static musl targets)
_build-target target:
    cargo build --release --target {{ target }} -p tokenburn -p tokenburn-tui -p tokenburn-web
    @if [ "{{ target }}" != "{{ MUSL_TARGET }}" ]; then cargo build --release --target {{ target }} -p tokenburn-gui; fi

# Fully static Linux binaries (CLI + TUI + web) that run on any distro. Needs musl-tools.
build-musl:
    rustup target add {{ MUSL_TARGET }}
    cargo build --release --target {{ MUSL_TARGET }} -p tokenburn -p tokenburn-tui -p tokenburn-web

# Build a portable .tar.gz / .zip with every binary (e.g. just package-archive 0.1.0 x86_64-unknown-linux-gnu)
package-archive version target suffix="": bundle-web-release (_build-target target)
    bash scripts/ci/package_archive.sh {{ version }} {{ target }} {{ suffix }}

# Build Linux .deb and .rpm packages (needs dpkg-deb + alien)
package-linux version: bundle-web-release (_build-target LINUX_TARGET)
    nu scripts/ci/package_linux.nu {{ version }} {{ LINUX_TARGET }}

# Build Linux AppImages for the TUI and GUI (requires appimagetool in PATH)
package-appimage version: (_build-target LINUX_TARGET)
    bash scripts/ci/package_appimage.sh {{ version }} {{ LINUX_TARGET }}

# Build the macOS DMG for this Mac's architecture (requires create-dmg for the styled window)
package-macos version:
    #!/usr/bin/env sh
    set -e
    TARGET="$(rustc -vV | sed -n 's/host: //p')"
    just bundle-web-release
    cargo build --release --target "$TARGET" -p tokenburn -p tokenburn-tui -p tokenburn-gui -p tokenburn-web
    bash packaging/macos/create_dmg.sh {{ version }}

# Cross-compile Windows .exe files + a .zip (needs mingw-w64: pacman -S mingw-w64-gcc / brew install mingw-w64)
package-windows version: bundle-web-release
    rustup target add x86_64-pc-windows-gnu
    cargo build --release -p tokenburn -p tokenburn-gui -p tokenburn-web -p tokenburn-tui --target x86_64-pc-windows-gnu
    mkdir -p dist
    cp target/x86_64-pc-windows-gnu/release/tokenburn.exe     dist/tokenburn-cli-x86_64-windows.exe
    cp target/x86_64-pc-windows-gnu/release/tokenburn-gui.exe dist/tokenburn-gui-x86_64-windows.exe
    cp target/x86_64-pc-windows-gnu/release/tokenburn-web.exe dist/tokenburn-web-x86_64-windows.exe
    cp target/x86_64-pc-windows-gnu/release/tokenburn-tui.exe dist/tokenburn-tui-x86_64-windows.exe
    bash scripts/ci/package_archive.sh {{ version }} x86_64-pc-windows-gnu .exe

# Update AUR PKGBUILD to a new version
update-aur version:
    bash scripts/ci/update_aur.sh {{ version }}

# Note: macOS DMG is built automatically by GitHub Actions (macos-latest runner)
# Note: Windows NSIS installer is built by GitHub Actions (windows-latest runner)
# Note: Linux aarch64 is cross-compiled by both GitHub Actions and Gitea CI
# ── Documentation ─────────────────────────────────────────────────────────────

# Generate and open docs for the CLI crate
doc-cli:
    cargo doc --no-deps -p tokenburn --open

# Generate and open docs for the web crate
doc-web:
    cargo doc --no-deps -p tokenburn-web --open

# Generate and open docs for the GUI crate
doc-gui:
    cargo doc --no-deps -p tokenburn-gui --open

# Generate and open docs for the TUI crate
doc-tui:
    cargo doc --no-deps -p tokenburn-tui --open

# Generate docs for the full workspace (no browser)
doc:
    cargo doc --no-deps --workspace

# ── Changelog ─────────────────────────────────────────────────────────────────

# Regenerate the full CHANGELOG.md from all tags
changelog: _check-git-cliff
    @echo "Generating full changelog…"
    git-cliff --output CHANGELOG.md
    @echo "✅ CHANGELOG.md updated."

# Prepend only unreleased commits to CHANGELOG.md
changelog-unreleased: _check-git-cliff
    git-cliff --unreleased --prepend CHANGELOG.md
    @echo "✅ Unreleased changes prepended."

# Preview changelog for the next release without writing the file
changelog-preview: _check-git-cliff
    @git-cliff --unreleased

# Show the latest tagged release entry (no file write)
changelog-latest: _check-git-cliff
    @git-cliff --latest

# ── Version bump ─────────────────────────────────────────────────────────────

# Validate that a version string will produce a valid vX.Y.Z tag.
validate-tag version: _check-nu
    @nu scripts/ci/validate_tag.nu "v{{ version }}" 2>&1 >/dev/null

# Fail fast if the requested version is the same as the current one.
_check-version-changed version: _check-nu
    #!/usr/bin/env sh
    current=$(nu scripts/version.nu)
    if [ "$current" = "{{ version }}" ]; then
        echo "❌ Version {{ version }} is already the current version. Nothing to bump."
        exit 1
    fi
    echo "✅ Version will change: $current → {{ version }}"

# Bump the workspace version, regenerate Cargo.lock + CHANGELOG.md, commit and tag.

# Order: validation (cheap) → quality gate (expensive) → ONLY THEN edit, commit and tag.
# A red gate leaves the tree untouched; a failure after the edits is rolled back.
bump version: (validate-tag version) (_check-version-changed version) check-release _check-git-cliff
    nu scripts/bump_version.nu --yes --skip-checks {{ version }}

# ── Publish (crates.io) ───────────────────────────────────────────────────────

# Run the full pre-publish readiness check (fmt, clippy, tests, docs, dry-run)
check-publish: _check-nu
    nu scripts/check_publish.nu

# Dry-run publish: core fully, the rest only packaged (they need core on crates.io first)
publish-dry: check-all
    @echo "Dry-run: tokenburn-core"
    cargo publish --dry-run -p tokenburn-core
    @for c in tokenburn tokenburn-gui tokenburn-web tokenburn-tui; do \
        echo "Package check: $c"; \
        cargo package --list --no-verify --allow-dirty -p $c > /dev/null || exit 1; \
    done
    @echo "✅ Dry run OK (dependents are verified for real once tokenburn-core is published)."

# Publish all three in dependency order: core → gui → tui.
publish: check-all publish-core publish-cli publish-gui publish-web publish-tui
    @echo "✅ tokenburn-core, tokenburn, tokenburn-gui, tokenburn-web and tokenburn-tui published to crates.io!"

# Publish tokenburn-core (required by gui and tui)
publish-core:
    @echo "📦 Publishing tokenburn-core…"
    cargo publish -p tokenburn-core
    @echo "⏳ Waiting 30 s for the index to propagate…"
    sleep 30

# Publish tokenburn (the clap CLI)
publish-cli:
    @echo "📦 Publishing tokenburn (CLI)…"
    cargo publish -p tokenburn

# Publish tokenburn-gui
publish-gui:
    @echo "📦 Publishing tokenburn-gui…"
    cargo publish -p tokenburn-gui

# Publish tokenburn-web
publish-web:
    @echo "📦 Publishing tokenburn-web…"
    cargo publish -p tokenburn-web

# Publish tokenburn-tui
publish-tui:
    @echo "📦 Publishing tokenburn-tui…"
    cargo publish -p tokenburn-tui

# Show what would be released without making any changes
release-preview: _check-git-cliff
    @echo "Current version: $(just version)"
    @echo ""
    @echo "Unreleased commits:"
    @git-cliff --unreleased
    @echo ""
    @echo "Workspace version:"
    @grep -A5 '^\[workspace\.package\]' Cargo.toml | grep '^version'
    @echo ""
    @echo "Published crates:  tokenburn (CLI)  •  tokenburn-gui (GUI)  •  tokenburn-web (web)  •  tokenburn-tui (TUI)"
    @echo "Internal crate:    tokenburn-core   (publish = false)"

# ── Housekeeping ──────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# Update all dependencies (Cargo.lock only)
update:
    cargo update

# Update dependencies, run the full quality gate, then commit and push if all green.
update-deps:
    @echo "⬆️  Updating dependencies…"
    cargo update
    @echo "🔍 Running quality gate…"
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated
    cargo test --workspace --locked --all-features --all-targets
    @echo "✅ All checks passed — committing dependency updates…"
    git add Cargo.lock
    git diff --cached --quiet || git commit -m "chore: update dependencies"
    git push origin "$(git branch --show-current)"
    @echo "✅ Dependency updates pushed to GitHub."

# Show outdated dependencies (requires cargo-outdated)
outdated:
    cargo outdated

# Show the current workspace version
version: _check-nu
    @nu scripts/version.nu

# Show all configured remotes
remotes:
    @git remote -v

# ── Git remotes & pushing ────────────────────────────────────────────────────
# tokenburn lives on GitHub (origin) and the Gitea Starscream instance. Every
# *-all recipe loops over REMOTES and keeps going when one fails.

REMOTES := "origin gitea-starscream"

# Push the current branch to GitHub (origin)
push:
    git push origin "$(git branch --show-current)"

# Push the current branch to Gitea Starscream
push-gitea-starscream:
    git push gitea-starscream "$(git branch --show-current)"

# Push the current branch to every remote (continues on failure)
push-all:
    #!/usr/bin/env sh
    failed=""; skipped=""
    for r in {{ REMOTES }}; do
        git remote get-url "$r" >/dev/null 2>&1 || { echo "↷ skipping $r (not configured here; add with: git remote add $r <url>)"; skipped="$skipped $r"; continue; }
        git push "$r" "$(git branch --show-current)" || failed="$failed $r"
    done
    if [ -n "$skipped" ]; then echo "ℹ️  Skipped (not configured):$skipped"; fi
    if [ -n "$failed" ]; then echo "⚠️  Failed to push to:$failed"; exit 1; fi
    echo "✅ Pushed to: {{ REMOTES }}"

# Force-push the current branch to every remote (leases are NOT used — be sure!)
push-all-force:
    #!/usr/bin/env sh
    failed=""; skipped=""
    for r in {{ REMOTES }}; do
        git remote get-url "$r" >/dev/null 2>&1 || { echo "↷ skipping $r (not configured here; add with: git remote add $r <url>)"; skipped="$skipped $r"; continue; }
        git push --force "$r" "$(git branch --show-current)" || failed="$failed $r"
    done
    if [ -n "$skipped" ]; then echo "ℹ️  Skipped (not configured):$skipped"; fi
    if [ -n "$failed" ]; then echo "⚠️  Failed to force-push to:$failed"; exit 1; fi
    echo "✅ Force-pushed to: {{ REMOTES }}"

# Pull the current branch from GitHub (origin)
pull:
    git pull origin "$(git branch --show-current)"

# Pull the current branch from Gitea Starscream
pull-gitea-starscream:
    git pull gitea-starscream "$(git branch --show-current)"

# Pull the current branch from every remote (continues on failure)
pull-all:
    #!/usr/bin/env sh
    failed=""; skipped=""
    for r in {{ REMOTES }}; do
        git remote get-url "$r" >/dev/null 2>&1 || { echo "↷ skipping $r (not configured here; add with: git remote add $r <url>)"; skipped="$skipped $r"; continue; }
        git pull "$r" "$(git branch --show-current)" || failed="$failed $r"
    done
    if [ -n "$skipped" ]; then echo "ℹ️  Skipped (not configured):$skipped"; fi
    if [ -n "$failed" ]; then echo "⚠️  Failed to pull from:$failed"; exit 1; fi
    echo "✅ Pulled from: {{ REMOTES }}"

# Push all tags to GitHub
push-tags:
    git push origin --tags

# Push all tags to every remote (continues on failure)
push-tags-all:
    #!/usr/bin/env sh
    failed=""; skipped=""
    for r in {{ REMOTES }}; do
        git remote get-url "$r" >/dev/null 2>&1 || { echo "↷ skipping $r (not configured here; add with: git remote add $r <url>)"; skipped="$skipped $r"; continue; }
        git push "$r" --tags || failed="$failed $r"
    done
    if [ -n "$skipped" ]; then echo "ℹ️  Skipped (not configured):$skipped"; fi
    if [ -n "$failed" ]; then echo "⚠️  Failed to push tags to:$failed"; exit 1; fi
    echo "✅ Tags pushed to: {{ REMOTES }}"

# Force-sync Gitea Starscream with GitHub (main + tags)
sync-gitea-starscream:
    git push gitea-starscream "$(git branch --show-current)" --force
    git push gitea-starscream --tags --force
    @echo "✅ Gitea Starscream force-synced."

# Add the Gitea Starscream remote if it is missing
setup-remotes:
    #!/usr/bin/env sh
    git remote get-url origin >/dev/null 2>&1 || git remote add origin git@github.com:sorinirimies/tokenburn.git
    git remote get-url gitea-starscream >/dev/null 2>&1 || git remote add gitea-starscream gitea@192.168.1.44:sorin/tokenburn.git
    git remote -v

# Add a Gitea remote and optionally push — interactive (nu script)
setup-gitea url: _check-nu
    nu scripts/setup_gitea.nu {{ url }}

# ── Release workflows ─────────────────────────────────────────────────────────
# Pushing a vX.Y.Z tag starts the Release workflow on whichever host receives it
# (GitHub: .github/workflows/release.yml, Gitea: .gitea/workflows/release.yml).

# First release of the version already in Cargo.toml (0.1.0): gate, changelog, tag, push to all remotes
release-initial: check-release _check-git-cliff _check-nu
    #!/usr/bin/env sh
    set -e
    version=$(nu scripts/version.nu)
    tag="v$version"
    nu scripts/ci/validate_tag.nu "$tag" >/dev/null
    if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
        echo "❌ Tag $tag already exists. Use 'just release <new-version>' to bump."; exit 1
    fi
    if [ -n "$(git status --porcelain)" ]; then
        echo "❌ Working tree is dirty — commit or stash first."; exit 1
    fi
    git-cliff --output CHANGELOG.md --tag "$tag"
    git add CHANGELOG.md
    git diff --cached --quiet || git commit -m "chore: release $tag"
    git tag -a "$tag" -m "Release $tag"
    echo "🏷️  Tagged $tag — pushing to: {{ REMOTES }}"
    failed=""; skipped=""
    for r in {{ REMOTES }}; do
        git remote get-url "$r" >/dev/null 2>&1 || { echo "↷ skipping $r (not configured here; add with: git remote add $r <url>)"; skipped="$skipped $r"; continue; }
        git push "$r" "$(git branch --show-current)" || failed="$failed $r"
        git push "$r" "$tag" || failed="$failed $r"
    done
    if [ -n "$skipped" ]; then echo "ℹ️  Skipped (not configured):$skipped"; fi
    if [ -n "$failed" ]; then echo "⚠️  Release $tag failed to push to:$failed"; exit 1; fi
    echo "✅ Release $tag pushed to: {{ REMOTES }} — Release workflows will start."

# Bump, commit, tag, then push to every remote — triggers the Release workflows.
release version: (bump version)
    #!/usr/bin/env sh
    failed=""; skipped=""
    for r in {{ REMOTES }}; do
        git remote get-url "$r" >/dev/null 2>&1 || { echo "↷ skipping $r (not configured here; add with: git remote add $r <url>)"; skipped="$skipped $r"; continue; }
        git push --follow-tags "$r" "$(git branch --show-current)" || failed="$failed $r"
    done
    if [ -n "$skipped" ]; then echo "ℹ️  Skipped (not configured):$skipped"; fi
    if [ -n "$failed" ]; then echo "⚠️  Release v{{ version }} failed to push to:$failed"; exit 1; fi
    echo "✅ Release v{{ version }} pushed to: {{ REMOTES }}"

# Bump, commit, tag, then push to GitHub only.
release-github version: (bump version)
    git push --follow-tags origin "$(git branch --show-current)"
    @echo "✅ Release v{{ version }} pushed to GitHub."

# Bump, commit, tag, then push to Gitea Starscream only.
release-gitea-starscream version: (bump version)
    git push --follow-tags gitea-starscream "$(git branch --show-current)"
    @echo "✅ Release v{{ version }} pushed to Gitea Starscream."

# Push the latest commit and all tags to every remote (no bump, continues on failure).
push-release-all: check-all
    #!/usr/bin/env sh
    failed=""; skipped=""
    for r in {{ REMOTES }}; do
        git remote get-url "$r" >/dev/null 2>&1 || { echo "↷ skipping $r (not configured here; add with: git remote add $r <url>)"; skipped="$skipped $r"; continue; }
        git push --follow-tags "$r" "$(git branch --show-current)" || failed="$failed $r"
    done
    if [ -n "$skipped" ]; then echo "ℹ️  Skipped (not configured):$skipped"; fi
    if [ -n "$failed" ]; then echo "⚠️  Failed to push to:$failed"; exit 1; fi
    echo "✅ Latest commit + tags pushed to: {{ REMOTES }}"

# Manually re-trigger the GitHub Release workflow for an existing tag via the gh CLI.
release-retrigger version:
    @command -v gh >/dev/null 2>&1 || { \
        echo "❌ GitHub CLI (gh) not found. Install from https://cli.github.com"; exit 1; \
    }
    @echo "Manually dispatching Release workflow for tag v{{ version }}…"
    gh workflow run release.yml --field tag=v{{ version }}
    @echo "✅ Dispatched — check progress at: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/actions"

# Run the nightly dependency update workflow on GitHub on demand.
deps-update-now:
    gh workflow run deps-update.yml

# Security: dependency vulnerability scan (needs `cargo install cargo-audit`)
audit:
    cargo audit

# Security: licenses, advisories, bans, sources (needs `cargo install cargo-deny`)
deny:
    cargo deny check
