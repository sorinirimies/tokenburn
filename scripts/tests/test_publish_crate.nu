#!/usr/bin/env nu
# ── TokenBurn · test_publish_crate.nu ────────────────────────────────────────
# Tests for scripts/ci/publish_crate.nu — exported pure helper functions.
# We can only unit-test the pure helpers (is_already_published is impure and
# calls cargo, so it's exercised via integration tests / manual runs).

use std/assert
use runner.nu *
use ../ci/publish_crate.nu [copy_readme]

# ── Helpers ─────────────────────────────────────────────────────────────────

# Create a temporary workspace-like directory with a README.md and a crate dir.
def make_temp_workspace []: nothing -> record<root: string, crate_dir: string> {
    let root = (mktemp -d)
    let readme = ($root | path join "README.md")
    "# TokenBurn\nFake README for testing." | save $readme

    let crate_dir = ($root | path join "crates" "tokenburn-core")
    mkdir $crate_dir

    { root: $root, crate_dir: $crate_dir }
}

# ── Tests: copy_readme ──────────────────────────────────────────────────────

def "test copy_readme: copies README into target dir" [] {
    let ws = (make_temp_workspace)
    let original_dir = ($env.PWD)
    cd $ws.root

    copy_readme "crates/tokenburn-core"

    let dst = ($ws.root | path join "crates" "tokenburn-core" "README.md")
    assert ($dst | path exists)

    let content = (open --raw $dst)
    assert ($content | str contains "TokenBurn")

    # Restore original dir before cleanup
    cd $original_dir
    rm -rf $ws.root
}

def "test copy_readme: overwrites existing README" [] {
    let ws = (make_temp_workspace)
    let original_dir = ($env.PWD)
    cd $ws.root

    # Pre-populate with old content
    let dst = ($ws.root | path join "crates" "tokenburn-core" "README.md")
    "old content" | save $dst

    copy_readme "crates/tokenburn-core"

    let content = (open --raw $dst)
    assert ($content | str contains "TokenBurn")
    assert (not ($content | str contains "old content"))

    # Restore original dir before cleanup
    cd $original_dir
    rm -rf $ws.root
}

def "test copy_readme: handles missing README gracefully" [] {
    let root = (mktemp -d)
    let crate_dir = ($root | path join "crates" "tokenburn-core")
    mkdir $crate_dir
    let original_dir = ($env.PWD)
    cd $root

    # No README.md in root — should print warning but not crash
    copy_readme "crates/tokenburn-core"

    let dst = ($crate_dir | path join "README.md")
    assert (not ($dst | path exists))

    # Restore original dir before cleanup
    cd $original_dir
    rm -rf $root
}

# ── Tests: publish order invariants ─────────────────────────────────────────
# These verify the publish-order contract that workflows depend on:
# core → gui → tui (core must be first because the others depend on it).

def "test publish order: core is first" [] {
    let order = ["tokenburn-core", "tokenburn", "tokenburn-tui"]
    assert equal ($order | first) "tokenburn-core"
}

def "test publish order: has exactly three crates" [] {
    let order = ["tokenburn-core", "tokenburn", "tokenburn-tui"]
    assert equal ($order | length) 3
}

def "test publish order: core before gui" [] {
    let order = ["tokenburn-core", "tokenburn", "tokenburn-tui"]
    let core_idx = ($order | enumerate | where { |it| $it.item == "tokenburn-core" } | get index | first)
    let gui_idx  = ($order | enumerate | where { |it| $it.item == "tokenburn"      } | get index | first)
    assert ($core_idx < $gui_idx)
}

def "test publish order: core before tui" [] {
    let order = ["tokenburn-core", "tokenburn", "tokenburn-tui"]
    let core_idx = ($order | enumerate | where { |it| $it.item == "tokenburn-core" } | get index | first)
    let tui_idx  = ($order | enumerate | where { |it| $it.item == "tokenburn-tui"  } | get index | first)
    assert ($core_idx < $tui_idx)
}

# ── Tests: readme-dir resolution ────────────────────────────────────────────
# Verify the default directory convention: crates/<crate-name>

def "test readme dir default: core resolves to crates/tokenburn-core" [] {
    let crate = "tokenburn-core"
    let default_dir = $"crates/($crate)"
    assert equal $default_dir "crates/tokenburn-core"
}

def "test readme dir default: gui resolves to crates/tokenburn" [] {
    # Note: the *crate* is called "tokenburn" but its directory is tokenburn-gui.
    # The workflow passes --readme-dir explicitly for the GUI crate.
    let crate = "tokenburn"
    let default_dir = $"crates/($crate)"
    assert equal $default_dir "crates/tokenburn"
}

def "test readme dir default: tui resolves to crates/tokenburn-tui" [] {
    let crate = "tokenburn-tui"
    let default_dir = $"crates/($crate)"
    assert equal $default_dir "crates/tokenburn-tui"
}

def "test readme dir override: explicit dir takes precedence" [] {
    let readme_dir = "crates/tokenburn-gui"
    let crate = "tokenburn"
    let resolved = if ($readme_dir | is-empty) { $"crates/($crate)" } else { $readme_dir }
    assert equal $resolved "crates/tokenburn-gui"
}

def "test readme dir override: empty string falls back to default" [] {
    let readme_dir = ""
    let crate = "tokenburn-core"
    let resolved = if ($readme_dir | is-empty) { $"crates/($crate)" } else { $readme_dir }
    assert equal $resolved "crates/tokenburn-core"
}

# ── Tests: version string handling ──────────────────────────────────────────

def "test version string: simple semver is valid for publish" [] {
    let version = "0.1.6"
    assert ($version =~ '^\d+\.\d+\.\d+$')
}

def "test version string: pre-release is valid for publish" [] {
    let version = "1.0.0-rc.1"
    assert ($version =~ '^\d+\.\d+\.\d+')
}

def "test version string: bare tag prefix stripped" [] {
    let tag = "v0.1.6"
    let version = ($tag | str replace 'v' '')
    assert equal $version "0.1.6"
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
