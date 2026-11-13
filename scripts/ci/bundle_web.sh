#!/usr/bin/env bash
set -euo pipefail
# Usage: ./scripts/ci/bundle_web.sh [out_dir]    (default: dist/web-assets)
#
# Bundles the Topcoat browser runtime (the script behind signals / live
# updates) so `tokenburn-web` can run in live mode. Without it the dashboard
# falls back to a static page.
#
# Asset IDs derive from crate name + source path + asset path (NOT from the
# compile target), so a bundle made by building the web crate for the *host*
# also matches a cross-compiled binary built from the same checkout on the
# same machine. That is why this runs in the same CI job as the target build.
#
# Requires the topcoat-cli that matches the topcoat runtime in Cargo.lock (0.10.x).
# A newer CLI (0.12) bundles asset ids the 0.10 runtime cannot resolve → panic at
# startup. Keep the pin in the workflows in step with `topcoat` in Cargo.lock.
# Requires: `cargo install topcoat-cli --version 0.10.0 --locked`
OUT="${1:-dist/web-assets}"

if ! command -v topcoat >/dev/null 2>&1; then
    echo "❌ topcoat CLI not found. Install: cargo install topcoat-cli --version 0.10.0 --locked"
    exit 1
fi

rm -rf "$OUT"
topcoat asset bundle --release --package tokenburn-web --out "$OUT"

if [ ! -f "$OUT/manifest.toml" ]; then
    echo "❌ bundle has no manifest.toml: $OUT"
    exit 1
fi
echo "✅ web assets bundled → $OUT ($(ls "$OUT" | wc -l | tr -d ' ') files)"
