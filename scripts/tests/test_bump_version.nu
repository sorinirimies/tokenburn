#!/usr/bin/env nu
# ── TokenBurn · test_bump_version.nu ─────────────────────────────────────────
# Tests for scripts/bump_version.nu — bumping versions across all crate manifests.

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

# Read the tokenburn-core dependency version from a workspace Cargo.toml string.
def read_core_dep_version [cargo_toml: string]: nothing -> string {
    let dep_line = (
        $cargo_toml
        | lines
        | where { |l| $l =~ '^tokenburn-core\s*=' }
        | first
    )
    $dep_line
    | parse --regex 'version\s*=\s*"(?P<ver>[^"]+)"'
    | get ver
    | first
}

# Update the workspace.package version in a Cargo.toml string.
def apply_workspace_version_update [cargo_toml: string, new_version: string]: nothing -> string {
    $cargo_toml
    | lines
    | each { |l|
        let trimmed = ($l | str trim)
        if ($trimmed | str starts-with 'version') and (not ($trimmed | str contains 'workspace = true')) and ($trimmed =~ '^version\s*=\s*"') {
            $'version = "($new_version)"'
        } else {
            $l
        }
    }
    | str join "\n"
}

# Update the tokenburn-core dependency version in a workspace Cargo.toml string.
def apply_core_dep_update [cargo_toml: string, new_version: string]: nothing -> string {
    $cargo_toml
    | lines
    | each { |l|
        if ($l =~ '^tokenburn-core\s*=') {
            $'tokenburn-core = { path = "crates/tokenburn-core", version = "($new_version)" }'
        } else {
            $l
        }
    }
    | str join "\n"
}

# Bump: apply both workspace version and core dep version updates.
def bump_version [cargo_toml: string, new_version: string]: nothing -> string {
    let step1 = (apply_workspace_version_update $cargo_toml $new_version)
    apply_core_dep_update $step1 $new_version
}

# ── Tests ───────────────────────────────────────────────────────────────────

def "test bump_version: patch bump updates workspace version" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "1.0.1")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "1.0.1"
}

def "test bump_version: minor bump updates workspace version" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "1.1.0")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "1.1.0"
}

def "test bump_version: major bump updates workspace version" [] {
    let cargo = (make_workspace_cargo "1.2.3")
    let bumped = (bump_version $cargo "2.0.0")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "2.0.0"
}

def "test bump_version: core dep version is updated" [] {
    let cargo = (make_workspace_cargo "0.1.0")
    let bumped = (bump_version $cargo "0.2.0")
    let dep_ver = (read_core_dep_version $bumped)
    assert equal $dep_ver "0.2.0"
}

def "test bump_version: workspace and core dep versions stay in sync" [] {
    let cargo = (make_workspace_cargo "3.0.0")
    let bumped = (bump_version $cargo "3.1.0")
    let ws_ver = (read_workspace_version $bumped)
    let dep_ver = (read_core_dep_version $bumped)
    assert equal $ws_ver $dep_ver
}

def "test bump_version: pre-release version" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "1.1.0-rc.1")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "1.1.0-rc.1"
}

def "test bump_version: pre-release core dep version" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "1.1.0-rc.1")
    let dep_ver = (read_core_dep_version $bumped)
    assert equal $dep_ver "1.1.0-rc.1"
}

def "test bump_version: idempotent when version unchanged" [] {
    let cargo = (make_workspace_cargo "0.5.0")
    let bumped = (bump_version $cargo "0.5.0")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "0.5.0"
}

def "test bump_version: crate manifests use workspace version" [] {
    # Crate Cargo.tomls should have version.workspace = true,
    # meaning they don't need individual updates — only the root does.
    let crate_cargo = (make_crate_cargo "tokenburn-core")
    assert ($crate_cargo | str contains "version.workspace = true")

    let crate_gui = (make_crate_cargo "tokenburn")
    assert ($crate_gui | str contains "version.workspace = true")

    let crate_tui = (make_crate_cargo "tokenburn-tui")
    assert ($crate_tui | str contains "version.workspace = true")
}

def "test bump_version: workspace members list is preserved" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "2.0.0")
    assert ($bumped | str contains '"crates/tokenburn-core"')
    assert ($bumped | str contains '"crates/tokenburn-gui"')
    assert ($bumped | str contains '"crates/tokenburn-tui"')
}

def "test bump_version: double bump produces correct version" [] {
    let cargo = (make_workspace_cargo "0.1.0")
    let first = (bump_version $cargo "0.2.0")
    let second = (bump_version $first "0.3.0")
    let ver = (read_workspace_version $second)
    let dep_ver = (read_core_dep_version $second)
    assert equal $ver "0.3.0"
    assert equal $dep_ver "0.3.0"
}

# ── Ordering guarantees (the script itself) ─────────────────────────────────

# Position of `needle` in the body of `def main` of bump_version.nu.
def main_pos [needle: string]: nothing -> int {
    let src = (open ($env.FILE_PWD | path join ".." "bump_version.nu") --raw)
    let body = ($src | split row "def main [" | last)
    ($body | str index-of $needle)
}

def "test bump_version: the quality gate runs before anything is edited" [] {
    let gate = (main_pos "run_checks")
    let first_edit = (main_pos "update_workspace_version $new_version")
    assert ($gate >= 0)
    assert ($first_edit > $gate) "the gate must come before the first edit"
}

def "test bump_version: commit and tag are the very last steps" [] {
    let commit = (main_pos '"commit"')
    let tag = (main_pos '"tag" "-a"')
    let lock = (main_pos '"cargo" "update"')
    let changelog = (main_pos "git-cliff")
    assert ($lock < $commit)
    assert ($changelog < $commit)
    assert ($commit < $tag)
}

def "test bump_version: every post-edit step is inside a rollback" [] {
    let edit = (main_pos "update_workspace_version $new_version")
    let try_at = (main_pos "try {")
    let catch_at = (main_pos "roll_back $start $tag")
    assert ($try_at >= 0 and $try_at < $edit) "edits happen inside try"
    assert ($catch_at > ((main_pos '"tag" "-a"'))) "rollback follows the last step"
}

def "test bump_version: refuses a dirty tree and an existing tag up front" [] {
    let dirty = (main_pos "status --porcelain")
    let tag_check = (main_pos "tag --list")
    let gate = (main_pos "run_checks")
    assert ($dirty >= 0 and $dirty < $gate)
    assert ($tag_check >= 0 and $tag_check < $gate)
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
