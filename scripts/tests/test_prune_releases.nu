#!/usr/bin/env nu
# ── TokenBurn · test_prune_releases.nu ───────────────────────────────────────
# Tests for scripts/ci/prune_releases.nu — which release files get deleted.

use std/assert
use runner.nu *
use ../ci/prune_releases.nu [assets_to_prune total_size]

def release [id: int, tag: string, assets: int]: nothing -> record {
    {
        id: $id
        tag_name: $tag
        assets: (0..<$assets | each {|i| { id: $i, name: $"($tag)-($i).bin", size: 1000 } })
    }
}

def releases []: nothing -> list {
    [(release 3 "v0.1.4" 3) (release 2 "v0.1.3" 2) (release 1 "v0.1.2" 1)]
}

def "test prune_releases: only the latest release keeps its files" [] {
    let doomed = (assets_to_prune (releases) "v0.1.4")
    assert equal ($doomed | length) 3
    assert equal ($doomed | where tag == "v0.1.4" | length) 0 "the kept release is untouched"
    assert equal ($doomed | get tag | uniq | sort) ["v0.1.2" "v0.1.3"]
    assert equal ($doomed | get size | math sum) 3000
}

def "test prune_releases: each asset carries what the delete call needs" [] {
    let first = (assets_to_prune (releases) "v0.1.4" | where tag == "v0.1.3" | first)
    assert equal $first.release 2
    assert equal $first.id 0
    assert equal $first.name "v0.1.3-0.bin"
}

def "test prune_releases: an unknown keep tag refuses instead of wiping everything" [] {
    let failed = (try { assets_to_prune (releases) "v9.9.9"; false } catch { true })
    assert $failed
}

def "test prune_releases: a single release has nothing to prune" [] {
    assert equal (assets_to_prune [(release 3 "v0.1.4" 3)] "v0.1.4" | length) 0
}

def "test prune_releases: releases without an asset list are fine" [] {
    let rels = [(release 2 "v0.1.4" 1) { id: 1, tag_name: "v0.1.3", assets: null } { id: 0, tag_name: "v0.1.2" }]
    assert equal (assets_to_prune $rels "v0.1.4" | length) 0
}

def "test prune_releases: the keep tag is matched exactly" [] {
    # v0.1.40 is a different release from v0.1.4.
    let rels = [(release 2 "v0.1.40" 2) (release 1 "v0.1.4" 1)]
    let doomed = (assets_to_prune $rels "v0.1.40")
    assert equal ($doomed | get tag | uniq) ["v0.1.4"]
}

def "test prune_releases: the freed size adds up, and is zero for nothing" [] {
    assert equal (total_size (assets_to_prune (releases) "v0.1.4")) 3000
    assert equal (total_size []) 0
}

def main [] { run-tests }
