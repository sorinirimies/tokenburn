#!/usr/bin/env bash
# Screenshots of the desktop GUI (macOS only — uses osascript + screencapture).
#
#   examples/capture/gui.sh <out-dir> [theme ...]
#
# Runs target/debug/tokenburn-gui on the fixture data (see examples/vhs/fixture.sh),
# one window per theme, and saves <out-dir>/gui-<theme>.png.
set -euo pipefail
[ "$(uname)" = Darwin ] || { echo "gui.sh needs macOS (screencapture)"; exit 1; }

root="$(cd "$(dirname "$0")/../.." && pwd)"
out="${1:?usage: gui.sh <out-dir> [theme ...]}"; shift || true
themes=("$@"); [ ${#themes[@]} -gt 0 ] || themes=("Default")
mkdir -p "$out"

bash "$root/examples/vhs/fixture.sh" >/dev/null
eval "$("$root/examples/vhs/fixture.sh" env)"

for theme in "${themes[@]}"; do
    slug="$(printf '%s' "$theme" | tr '[:upper:] ' '[:lower:]-' | tr -cd 'a-z0-9-')"
    "$root/target/debug/tokenburn-gui" --daily --all --theme "$theme" >/dev/null 2>&1 &
    pid=$!
    sleep 5
    osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null 2>&1 || true
    sleep 1
    read -r x y w h < <(osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid) to get {position, size} of window 1" | tr -d ',' | awk '{print $1,$2,$3,$4}')
    screencapture -x -R "$x,$y,$w,$h" "$out/gui-$slug.png"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    echo "wrote $out/gui-$slug.png"
done
