#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# TokenBurn — keep only the LATEST release's files on a Gitea instance
# ──────────────────────────────────────────────────────────────────────────────
# Release assets (installers, AppImages, archives: ~40 files per release) are what
# fills a self-hosted Gitea. This deletes the assets of every release except the
# one named by --keep and leaves the releases themselves (notes, tags) alone.
#
# Usage:
#   nu scripts/ci/prune_releases.nu --server http://host:3000 --repo owner/name \
#       --keep v0.1.4 --token $TOKEN [--dry-run]
# ──────────────────────────────────────────────────────────────────────────────

# Which assets to delete: those of every release whose tag is not `keep`.
# Fails when `keep` is not a known release, so a typo can never wipe everything.
export def assets_to_prune [releases: list, keep: string]: nothing -> list {
    if ($releases | where tag_name == $keep | is-empty) {
        error make --unspanned { msg: $"release ($keep) not found; refusing to prune" }
    }
    $releases
    | where tag_name != $keep
    | each {|r|
        $r.assets? | default [] | each {|a|
            {
                release: $r.id
                tag: $r.tag_name
                id: $a.id
                name: $a.name
                size: ($a.size? | default 0)
            }
        }
    }
    | flatten
}

# Total bytes of a list of assets (0 for an empty list).
export def total_size [assets: list]: nothing -> int {
    $assets | each {|a| $a.size } | append 0 | math sum
}

def headers [token: string]: nothing -> list<string> {
    if ($token | is-empty) { [] } else { ["Authorization" $"token ($token)"] }
}

# Every release, across pages.
def fetch_releases [server: string, repo: string, token: string]: nothing -> list {
    mut all = []
    mut page = 1
    loop {
        let url = $"($server)/api/v1/repos/($repo)/releases?limit=50&page=($page)"
        let batch = (http get --headers (headers $token) $url)
        if ($batch | is-empty) { break }
        $all = ($all | append $batch)
        $page += 1
    }
    $all
}

def main [
    --server: string,   # Gitea base URL, e.g. http://host:3000
    --repo: string,     # owner/name
    --keep: string,     # tag whose files stay (the latest release)
    --token: string = "", # API token
    --dry-run,          # list what would be deleted, delete nothing
] {
    let server = ($server | str trim --right --char "/")
    let doomed = (assets_to_prune (fetch_releases $server $repo $token) $keep)

    for a in $doomed {
        let verb = if $dry_run { "would delete" } else { "deleting" }
        print $"($verb) ($a.tag)/($a.name) \(($a.size) bytes\)"
        if not $dry_run {
            http delete --headers (headers $token) $"($server)/api/v1/repos/($repo)/releases/($a.release)/assets/($a.id)" | ignore
        }
    }

    let freed = (total_size $doomed)
    let verb = if $dry_run { "would free" } else { "freed" }
    print $"($doomed | length) old release file\(s); ($verb) (($freed / 1_000_000) | math round --precision 1) MB; ($keep) keeps its files"
}
