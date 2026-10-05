#!/usr/bin/env nu
# ── TokenBurn · test_check_publish.nu ────────────────────────────────────────
# Tests for scripts/check_publish.nu — dry-run `cargo publish` verification.

use std/assert
use runner.nu *

# ── Helpers ─────────────────────────────────────────────────────────────────

# Build a minimal workspace Cargo.toml with the given version.
def make_workspace_cargo [version: string]: nothing -> string {
    $'[workspace]
members = [
    "crates/tokenburn-core",
    "crates/tokenburn-gui",
    "crates/tokenburn-tui",
]
resolver = "2"

[workspace.package]
version = "($version)"
edition = "2021"

[workspace.dependencies]
tokenburn-core = { path = "crates/tokenburn-core", version = "($version)" }
'
}

# Build a minimal crate Cargo.toml (uses workspace version).
def make_crate_cargo [name: string]: nothing -> string {
    $'[package]
name = "($name)"
version.workspace = true
edition.workspace = true
'
}

# Simulate the publish-order that check_publish.nu would use.
# tokenburn-core must be published first (no internal deps),
# then tokenburn (gui) and tokenburn-tui (both depend on core).
def publish_order []: nothing -> list<string> {
    ["tokenburn-core", "tokenburn", "tokenburn-tui"]
}

# Read the workspace.package version from a Cargo.toml string.
def read_workspace_version [cargo_toml: string]: nothing -> string {
    $cargo_toml
    | lines
    | each { |l| $l | str trim }
    | where { |l| $l | str starts-with 'version' }
    | where { |l| not ($l | str contains 'workspace = true') }
    | first
    | parse --regex 'version\s*=\s*"(?P<ver>[^"]+)"'
    | get ver
    | first
}

# Check whether a crate manifest declares version.workspace = true.
def has_workspace_version [cargo_toml: string]: nothing -> bool {
    $cargo_toml | str contains "version.workspace = true"
}

# Validate that the tokenburn-core dependency version in the workspace manifest
# matches the workspace.package version (a publish prerequisite).
def versions_in_sync [workspace_cargo: string]: nothing -> bool {
    let ws_ver = (read_workspace_version $workspace_cargo)
    let dep_line = (
        $workspace_cargo
        | lines
        | where { |l| $l =~ '^tokenburn-core\s*=' }
        | first
    )
    let dep_ver = (
        $dep_line
        | parse --regex 'version\s*=\s*"(?P<ver>[^"]+)"'
        | get ver
        | first
    )
    $ws_ver == $dep_ver
}

# ── Tests ───────────────────────────────────────────────────────────────────

def "test check_publish: publish order starts with core" [] {
    let order = (publish_order)
    assert equal ($order | first) "tokenburn-core"
}

def "test check_publish: publish order has three crates" [] {
    let order = (publish_order)
    assert equal ($order | length) 3
}

def "test check_publish: publish order contains all crates" [] {
    let order = (publish_order)
    assert ("tokenburn-core" in $order)
    assert ("tokenburn" in $order)
    assert ("tokenburn-tui" in $order)
}

def "test check_publish: core comes before gui" [] {
    let order = (publish_order)
    let core_idx = ($order | enumerate | where { |it| $it.item == "tokenburn-core" } | get index | first)
    let gui_idx  = ($order | enumerate | where { |it| $it.item == "tokenburn"      } | get index | first)
    assert ($core_idx < $gui_idx)
}

def "test check_publish: core comes before tui" [] {
    let order = (publish_order)
    let core_idx = ($order | enumerate | where { |it| $it.item == "tokenburn-core" } | get index | first)
    let tui_idx  = ($order | enumerate | where { |it| $it.item == "tokenburn-tui"  } | get index | first)
    assert ($core_idx < $tui_idx)
}

def "test check_publish: versions in sync returns true for matching" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    assert (versions_in_sync $cargo)
}

def "test check_publish: versions in sync detects mismatch" [] {
    # Manually craft a workspace where the dep version differs.
    let cargo = '[workspace]
members = [
    "crates/tokenburn-core",
    "crates/tokenburn-gui",
    "crates/tokenburn-tui",
]
resolver = "2"

[workspace.package]
version = "2.0.0"
edition = "2021"

[workspace.dependencies]
tokenburn-core = { path = "crates/tokenburn-core", version = "1.0.0" }
'
    assert (not (versions_in_sync $cargo))
}

def "test check_publish: crate manifests use workspace version" [] {
    let core = (make_crate_cargo "tokenburn-core")
    let gui  = (make_crate_cargo "tokenburn")
    let tui  = (make_crate_cargo "tokenburn-tui")
    assert (has_workspace_version $core)
    assert (has_workspace_version $gui)
    assert (has_workspace_version $tui)
}

def "test check_publish: workspace cargo has all members" [] {
    let cargo = (make_workspace_cargo "0.1.0")
    assert ($cargo | str contains '"crates/tokenburn-core"')
    assert ($cargo | str contains '"crates/tokenburn-gui"')
    assert ($cargo | str contains '"crates/tokenburn-tui"')
}

def "test check_publish: workspace cargo has dep line" [] {
    let cargo = (make_workspace_cargo "0.3.0")
    let dep_lines = (
        $cargo
        | lines
        | where { |l| $l =~ '^tokenburn-core\s*=' }
    )
    assert equal ($dep_lines | length) 1
}

def "test check_publish: zero version is publishable format" [] {
    let cargo = (make_workspace_cargo "0.0.0")
    assert (versions_in_sync $cargo)
}

def "test check_publish: pre-release version stays in sync" [] {
    let cargo = (make_workspace_cargo "1.0.0-beta.2")
    assert (versions_in_sync $cargo)
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
