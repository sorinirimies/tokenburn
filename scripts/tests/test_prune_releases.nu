#!/usr/bin/env nu
# ── TokenBurn · test_prune_releases.nu ───────────────────────────────────────
# Runs the Python tests of scripts/ci/prune_releases.py (fake Gitea API, no network).

use std/assert
use runner.nu *

def "test prune_releases: only the latest release keeps its files" [] {
    let r = (do { ^python3 ($env.FILE_PWD | path join "test_prune_releases.py") } | complete)
    assert equal $r.exit_code 0 $"python tests failed:\n($r.stderr)"
}

def main [] { run-tests }
