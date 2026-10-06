#!/usr/bin/env python3
"""Keep only the LATEST release's files on a Gitea instance.

Release assets (installers, AppImages, archives: ~40 files per release) are what
fills a self-hosted Gitea. This deletes the assets of every release except the one
named by --keep, and leaves the releases themselves (notes, tags) alone.

    prune_releases.py --server http://host:3000 --repo owner/name --keep v0.1.4 \
        --token "$TOKEN" [--dry-run]
"""
import argparse
import json
import sys
import urllib.request


def api(server, token, method, path):
    req = urllib.request.Request(
        f"{server}/api/v1{path}",
        method=method,
        headers={"Authorization": f"token {token}"} if token else {},
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        body = resp.read()
    return json.loads(body) if body else None


def releases(server, token, repo):
    """Every release, newest first, across pages."""
    page = 1
    while True:
        batch = api(server, token, "GET", f"/repos/{repo}/releases?limit=50&page={page}")
        if not batch:
            return
        yield from batch
        page += 1


def prune(server, token, repo, keep, dry_run=False, log=print):
    """Delete the assets of every release whose tag is not `keep`.

    Returns (assets_deleted, bytes_freed). Refuses to do anything when `keep` is not
    a known release, so a typo can never wipe everything.
    """
    all_releases = list(releases(server, token, repo))
    if not any(r["tag_name"] == keep for r in all_releases):
        raise SystemExit(f"release {keep} not found on {repo}; refusing to prune")
    deleted = freed = 0
    for rel in all_releases:
        if rel["tag_name"] == keep:
            continue
        for asset in rel.get("assets") or []:
            log(f"{'would delete' if dry_run else 'deleting'} {rel['tag_name']}/{asset['name']} ({asset.get('size', 0)} bytes)")
            if not dry_run:
                api(server, token, "DELETE", f"/repos/{repo}/releases/{rel['id']}/assets/{asset['id']}")
            deleted += 1
            freed += asset.get("size", 0)
    return deleted, freed


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--server", required=True)
    p.add_argument("--repo", required=True)
    p.add_argument("--keep", required=True, help="tag whose files stay (the latest release)")
    p.add_argument("--token", default="")
    p.add_argument("--dry-run", action="store_true")
    a = p.parse_args(argv)
    deleted, freed = prune(a.server.rstrip("/"), a.token, a.repo, a.keep, a.dry_run)
    verb = "would free" if a.dry_run else "freed"
    print(f"{deleted} old release file(s); {verb} {freed / 1e6:.1f} MB; {a.keep} keeps its files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
