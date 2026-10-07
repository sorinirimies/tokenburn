#!/usr/bin/env nu
# ── TokenBurn · test_lock_changes.nu ─────────────────────────────────────────
# Tests for scripts/ci/lock_changes.nu — "did the nightly update REALLY upgrade
# something?" (no release for downgrades, checksum-only changes or reshuffles).

use std/assert
use runner.nu *
use ../ci/lock_changes.nu [cmp_version parse_tree classify should_release]

def pkg [name: string, version: string]: nothing -> record { { name: $name, version: $version } }

# ── version comparison ──────────────────────────────────────────────────────

def "test lock_changes: versions compare numerically, not as text" [] {
    assert equal (cmp_version "1.9.0" "1.10.0") (-1)
    assert equal (cmp_version "0.8.60" "0.8.59") 1
    assert equal (cmp_version "2.0.0" "10.0.0") (-1)
    assert equal (cmp_version "1.2.3" "1.2.3") 0
}

def "test lock_changes: a pre-release sorts before its release" [] {
    assert equal (cmp_version "1.0.0-rc.1" "1.0.0") (-1)
    assert equal (cmp_version "1.0.0" "1.0.0-rc.1") 1
    assert equal (cmp_version "1.0.0-alpha" "1.0.0-beta") (-1)
    assert equal (cmp_version "1.0.0-rc.1" "1.0.0-rc.1") 0
}

def "test lock_changes: build metadata and a leading v are ignored" [] {
    assert equal (cmp_version "1.0.0+abc" "1.0.0+def") 0
    assert equal (cmp_version "v1.2.0" "1.1.9") 1
    assert equal (cmp_version "0.5.0+11769913" "0.5.0") 0
}

def "test lock_changes: missing parts count as zero" [] {
    assert equal (cmp_version "1.2" "1.2.0") 0
    assert equal (cmp_version "1.2.1" "1.2") 1
}

# ── parsing ─────────────────────────────────────────────────────────────────

def "test lock_changes: cargo tree lines are parsed and annotations dropped" [] {
    let text = "serde v1.0.228\nserde v1.0.228 (*)\nzerocopy-derive v0.8.60 (proc-macro)\n\nobjc2 v0.6.5\n"
    let pkgs = (parse_tree $text)
    assert equal $pkgs [(pkg "serde" "1.0.228") (pkg "zerocopy-derive" "0.8.60") (pkg "objc2" "0.6.5")]
}

# ── classification and the release decision ─────────────────────────────────

def "test lock_changes: an upgrade is a release" [] {
    let c = (classify [(pkg "hyper" "1.11.1") (pkg "serde" "1.0.0")] [(pkg "hyper" "1.12.0") (pkg "serde" "1.0.0")])
    assert equal ($c.upgraded | length) 1
    assert equal ($c.upgraded | first) { name: "hyper", from: "1.11.1", to: "1.12.0" }
    assert (should_release $c)
}

def "test lock_changes: nothing changed is not a release" [] {
    let same = [(pkg "a" "1.0.0") (pkg "b" "2.0.0")]
    let c = (classify $same ($same | reverse))
    assert equal ($c.upgraded | length) 0
    assert (not (should_release $c))
}

def "test lock_changes: a downgrade is never released" [] {
    let c = (classify [(pkg "a" "1.2.0")] [(pkg "a" "1.1.9")])
    assert equal ($c.downgraded | length) 1
    assert (not (should_release $c))
}

def "test lock_changes: one downgrade vetoes even a real upgrade" [] {
    let c = (classify [(pkg "a" "1.0.0") (pkg "b" "2.0.0")] [(pkg "a" "1.1.0") (pkg "b" "1.9.0")])
    assert equal ($c.upgraded | length) 1
    assert equal ($c.downgraded | length) 1
    assert (not (should_release $c)) "a human should look at a downgrade"
}

def "test lock_changes: a crate present in two versions is compared version by version" [] {
    let before = [(pkg "objc2" "0.5.2") (pkg "objc2" "0.6.4")]
    let after = [(pkg "objc2" "0.5.3") (pkg "objc2" "0.6.5")]
    let c = (classify $before $after)
    assert equal ($c.upgraded | length) 2
    assert equal ($c.downgraded | length) 0
    assert (should_release $c)
}

def "test lock_changes: one of two versions upgrading leaves the other alone" [] {
    let c = (classify [(pkg "objc2" "0.5.2") (pkg "objc2" "0.6.4")] [(pkg "objc2" "0.5.2") (pkg "objc2" "0.6.5")])
    assert equal ($c.upgraded | length) 1
    assert equal ($c.upgraded | first | get from) "0.6.4"
}

def "test lock_changes: added and removed packages alone never trigger a release" [] {
    let c = (classify [(pkg "old" "1.0.0")] [(pkg "new" "1.0.0")])
    assert equal ($c.added | length) 1
    assert equal ($c.removed | length) 1
    assert equal ($c.upgraded | length) 0
    assert (not (should_release $c))
}

def "test lock_changes: a new dependency next to a real upgrade still releases" [] {
    let c = (classify [(pkg "a" "1.0.0")] [(pkg "a" "1.0.1") (pkg "extra" "0.1.0")])
    assert equal ($c.added | length) 1
    assert (should_release $c)
}

def "test lock_changes: a second version appearing is an addition, not an upgrade" [] {
    let c = (classify [(pkg "a" "1.0.0")] [(pkg "a" "1.0.0") (pkg "a" "2.0.0")])
    assert equal ($c.added | length) 1
    assert equal ($c.upgraded | length) 0
    assert (not (should_release $c))
}

def "test lock_changes: ten is greater than nine" [] {
    assert (should_release (classify [(pkg "a" "0.9.0")] [(pkg "a" "0.10.0")]))
    assert (not (should_release (classify [(pkg "a" "0.10.0")] [(pkg "a" "0.9.0")])))
}

# ── the script end to end (files in, GITHUB_OUTPUT-style file out) ───────────

def run_script [before: string, after: string]: nothing -> record {
    let dir = (mktemp --directory)
    $before | save ($dir | path join "before.txt")
    $after | save ($dir | path join "after.txt")
    let out = ($dir | path join "out.txt")
    let r = (^nu ($env.FILE_PWD | path join ".." "ci" "lock_changes.nu") ($dir | path join "before.txt") ($dir | path join "after.txt") --output $out | complete)
    let outputs = (open $out --raw | lines | each {|l| let kv = ($l | split row "=" ); { k: $kv.0, v: $kv.1 } } | reduce -f {} {|it, acc| $acc | upsert $it.k $it.v })
    rm -rf $dir
    { exit: $r.exit_code, stdout: $r.stdout, outputs: $outputs }
}

def "test lock_changes: the script reports a release for a real upgrade" [] {
    let r = (run_script "serde v1.0.1\nzerocopy v0.8.59\n" "serde v1.0.1\nzerocopy v0.8.60\n")
    assert equal $r.exit 0
    assert equal $r.outputs.changed "true"
    assert equal $r.outputs.upgraded "1"
    assert ($r.stdout | str contains "zerocopy 0.8.59 → 0.8.60")
}

def "test lock_changes: the script refuses a downgrade and says so" [] {
    let r = (run_script "a v1.2.0\n" "a v1.1.0\n")
    assert equal $r.outputs.changed "false"
    assert equal $r.outputs.downgraded "1"
    assert ($r.stdout | str contains "DOWNGRADE")
}

def "test lock_changes: the script says nothing moved when nothing moved" [] {
    let r = (run_script "a v1.0.0\n" "a v1.0.0 (*)\n")
    assert equal $r.outputs.changed "false"
    assert ($r.stdout | str contains "No package moved to a higher version")
}

def main [] { run-tests }
