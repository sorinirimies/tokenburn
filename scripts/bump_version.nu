#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# TokenBurn — Bump workspace version
# ──────────────────────────────────────────────────────────────────────────────
# Usage:
#   nu scripts/bump_version.nu <new_version>
#
# Example:
#   nu scripts/bump_version.nu 0.2.0
#
# What it does:
#   1. Validates the supplied semantic version string, and rejects one that is
#      equal to or older than the current version.
#   2. Updates `workspace.package.version` in the root Cargo.toml.
#   3. Updates the `tokenburn-core` dependency version wherever it appears.
#   4. Updates the version badge in README.md (if present).
#   5. Generates / updates the CHANGELOG via git-cliff (if installed).
#   6. Creates a Git commit and an annotated tag.
#
# Order matters: the quality gate (fmt, clippy, tests) runs FIRST, on the tree as
# it is, and nothing is edited, committed or tagged until it is green. If any later
# step fails, everything the script touched is rolled back, so a failed bump never
# leaves a half-bumped version behind.
#
# Flags:
#   --check       only validate the version (well-formed and strictly newer), then exit
#   --yes          non-interactive (CI)
#   --skip-checks  the caller already ran the gate (`just bump` runs check-release;
#                  the nightly workflows verify on every platform before releasing)
# ──────────────────────────────────────────────────────────────────────────────

# ── Helpers ───────────────────────────────────────────────────────────────────

# Validate that a string looks like a semver (MAJOR.MINOR.PATCH with optional pre-release).
def validate_version [version: string] {
    let pattern = '^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$'
    if ($version | find --regex $pattern | is-empty) {
        print $"(ansi red)Error:(ansi reset) '($version)' is not a valid semantic version."
        exit 1
    }
}

# Compare two validated versions: 1 if a > b, -1 if a < b, 0 if equal (semver precedence:
# MAJOR.MINOR.PATCH numerically, a pre-release sorts below its release, pre-release
# identifiers compare one by one, numeric ones numerically and below alphanumeric ones).
def compare_versions [a: string, b: string] {
    let pa = ($a | split row "-" | first 1 | get 0 | split row "." | each {|x| $x | into int })
    let pb = ($b | split row "-" | first 1 | get 0 | split row "." | each {|x| $x | into int })
    for i in 0..2 {
        if ($pa | get $i) > ($pb | get $i) { return 1 }
        if ($pa | get $i) < ($pb | get $i) { return (-1) }
    }
    let ra = ($a | split row "-" | skip 1 | str join "-")
    let rb = ($b | split row "-" | skip 1 | str join "-")
    if $ra == $rb { return 0 }
    if ($ra | is-empty) { return 1 }
    if ($rb | is-empty) { return (-1) }
    let ia = ($ra | split row ".")
    let ib = ($rb | split row ".")
    for i in 0..(([($ia | length) ($ib | length)] | math min) - 1) {
        let x = ($ia | get $i)
        let y = ($ib | get $i)
        if $x == $y { continue }
        let xn = ($x =~ '^\d+$')
        let yn = ($y =~ '^\d+$')
        if $xn and $yn { return (if ($x | into int) > ($y | into int) { 1 } else { -1 }) }
        if $xn { return (-1) }
        if $yn { return 1 }
        return (if $x > $y { 1 } else { -1 })
    }
    if ($ia | length) > ($ib | length) { 1 } else if ($ia | length) < ($ib | length) { -1 } else { 0 }
}

# Reject a version that is not strictly newer than the current one.
def require_newer [current: string, new: string] {
    let c = (compare_versions $new $current)
    if $c == 0 {
        print $"(ansi red)Error:(ansi reset) ($new) is already the current version."
        exit 1
    }
    if $c < 0 {
        print $"(ansi red)Error:(ansi reset) ($new) is older than the current version ($current); a bump must go up."
        exit 1
    }
}

# Replace the workspace.package.version line in the root Cargo.toml.
def update_workspace_version [version: string] {
    let cargo = (open Cargo.toml --raw)
    let updated = ($cargo | str replace --regex 'version\s*=\s*"[^"]+"' $'version      = "($version)"' )
    $updated | save --force Cargo.toml
    print $"(ansi green)✓(ansi reset) Updated workspace.package.version → ($version)"
}

# Update the tokenburn-core dependency version in the root Cargo.toml.
def update_core_dep_version [version: string] {
    let cargo = (open Cargo.toml --raw)
    let lines = ($cargo | lines)
    let updated_lines = ($lines | each {|line|
        if ($line | find --regex '^tokenburn-core\s*=' | is-not-empty) {
            $line | str replace --regex 'version\s*=\s*"[^"]+"' $'version = "($version)"'
        } else {
            $line
        }
    })
    $updated_lines | str join "\n" | save --force Cargo.toml
    print $"(ansi green)✓(ansi reset) Updated tokenburn-core dependency → ($version)"
}

# Update the crates.io version badge in README.md (if the badge exists).
def update_readme_badge [version: string] {
    if not ("README.md" | path exists) {
        print $"(ansi yellow)⚠(ansi reset) README.md not found — skipping badge update."
        return
    }
    let readme = (open README.md --raw)
    if ($readme =~ 'version-[0-9]+\.[0-9]+\.[0-9]+-blue') {
        let updated = (
            $readme
            | str replace --all --regex 'version-[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?-blue' $"version-($version)-blue"
        )
        $updated | save --force README.md
        print $"(ansi green)✓(ansi reset) Updated README.md version badge."
    } else {
        print $"(ansi yellow)⚠(ansi reset) No version badge found in README.md — skipping."
    }
}

# ── Main ──────────────────────────────────────────────────────────────────────

# Files this script edits (and therefore commits and, on failure, restores).
const TOUCHED = ["Cargo.toml" "Cargo.lock" "README.md" "CHANGELOG.md"]

# The quality gate. Runs on the unmodified tree; any failure aborts the bump
# before a single file has been changed.
def run_checks [] {
    print ""
    print $"(ansi cyan)── quality gate: nothing is bumped until it is green ──────────(ansi reset)"
    run-external "cargo" "fmt" "--all" "--" "--check"
    print $"(ansi green)✓(ansi reset) cargo fmt --check passed."
    run-external "cargo" "clippy" "--workspace" "--all-targets" "--all-features" "--" "-D" "warnings" "-A" "deprecated"
    print $"(ansi green)✓(ansi reset) cargo clippy passed."
    run-external "cargo" "test" "--workspace"
    print $"(ansi green)✓(ansi reset) cargo test passed."
}

# Undo everything the bump did: drop the commit and tag if they were made, restore the files.
def roll_back [start: string, tag: string] {
    print $"(ansi red)✗ Bump failed — rolling back to ($start | str substring 0..7).(ansi reset)"
    do --ignore-errors { run-external "git" "tag" "-d" $tag | ignore }
    run-external "git" "reset" "--hard" $start
    print $"(ansi yellow)Nothing was committed or tagged; the version is unchanged.(ansi reset)"
}

def main [
    new_version: string,  # New version in X.Y.Z format
    --yes (-y),           # Skip confirmation prompt (non-interactive)
    --skip-checks,        # The caller already ran the quality gate
    --check,              # Only validate the version (valid, and newer than current), then exit
] {
    print ""
    print $"(ansi cyan)══════════════════════════════════════════════════════════════(ansi reset)"
    print $"(ansi cyan)  TokenBurn — Bump Version(ansi reset)"
    print $"(ansi cyan)══════════════════════════════════════════════════════════════(ansi reset)"
    print ""

    # 0. Read current version
    let current_version = (open Cargo.toml | get workspace.package.version)
    let tag = $"v($new_version)"
    print $"  Current version : (ansi yellow)($current_version)(ansi reset)"
    print $"  New version     : (ansi green)($new_version)(ansi reset)"
    print ""

    # 1. Validate — everything that can be known up front, before any work.
    validate_version $new_version
    require_newer $current_version $new_version
    print $"(ansi green)✓(ansi reset) Version string validated and newer than ($current_version)."
    if $check { return }

    if (^git tag --list $tag | str trim | is-not-empty) {
        print $"(ansi red)Error:(ansi reset) tag ($tag) already exists."
        exit 1
    }
    # The script commits only the files it edits, but a rollback resets the tree,
    # so it must start clean or it would throw away someone's work.
    if (^git status --porcelain | str trim | is-not-empty) {
        print $"(ansi red)Error:(ansi reset) the working tree is dirty — commit or stash first."
        exit 1
    }
    let start = (^git rev-parse HEAD | str trim)

    # 2. Quality gate FIRST. Red => exit before anything is touched.
    if $skip_checks {
        print $"(ansi yellow)⚠(ansi reset) --skip-checks: assuming the caller already ran the gate."
    } else {
        run_checks
    }

    # 3. Only now edit, commit and tag; any failure rolls it all back.
    try {
        update_workspace_version $new_version
        update_core_dep_version $new_version
        update_readme_badge $new_version

        # Cargo.lock must carry the new member versions.
        print ""
        print $"(ansi cyan)── Cargo.lock ──────────────────────────────────────────────(ansi reset)"
        run-external "cargo" "update" "-p" "tokenburn-core" "-p" "tokenburn" "-p" "tokenburn-gui" "-p" "tokenburn-web" "-p" "tokenburn-tui"
        run-external "cargo" "metadata" "--locked" "--format-version" "1" | ignore
        print $"(ansi green)✓(ansi reset) Cargo.lock updated and consistent."

        print ""
        print $"(ansi cyan)── changelog ───────────────────────────────────────────────(ansi reset)"
        if (which git-cliff | is-not-empty) {
            run-external "git-cliff" "--output" "CHANGELOG.md" "--tag" $tag
            print $"(ansi green)✓(ansi reset) CHANGELOG.md updated via git-cliff."
        } else {
            print $"(ansi yellow)⚠(ansi reset) git-cliff not found — skipping changelog generation."
        }

        print ""
        print $"(ansi cyan)── git commit & tag ────────────────────────────────────────(ansi reset)"
        run-external "git" "add" ...$TOUCHED
        run-external "git" "commit" "-m" $"chore: bump version to ($new_version)"
        run-external "git" "tag" "-a" $tag "-m" $"Release ($tag)"
        print $"(ansi green)✓(ansi reset) Committed and tagged ($tag)."
    } catch {|err|
        roll_back $start $tag
        error make --unspanned { msg: $"bump to ($new_version) failed and was rolled back: ($err.msg)" }
    }

    print ""
    print $"(ansi green)══════════════════════════════════════════════════════════════(ansi reset)"
    print $"(ansi green)  TokenBurn version bumped to ($new_version) 🚀(ansi reset)"
    print $"(ansi green)══════════════════════════════════════════════════════════════(ansi reset)"
    print ""
    print "  Next steps:"
    print $"    git push origin main --tags"
    print ""
}
