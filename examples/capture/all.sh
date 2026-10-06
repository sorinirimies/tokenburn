#!/usr/bin/env bash
# Regenerate every GUI / web screenshot and the GIFs built from them.
# (The terminal GIFs come from `just vhs-all`; VHS cannot record a GUI or a browser.)
#
# Needs: a debug build of tokenburn-gui and tokenburn-web, the topcoat asset bundle
# (`topcoat asset bundle --package tokenburn-web --out target/debug/assets`), ffmpeg,
# node >= 22 and a Chromium-based browser (BROWSER=/path/to/it to override).
# The GUI half needs macOS; on other systems it is skipped.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
gen="$root/examples/vhs/generated"
shots="$(mktemp -d)"; trap 'rm -rf "$shots"' EXIT
mkdir -p "$gen"

THEMES=("Tokyo Night" "Catppuccin Mocha" "Solarized Light" "Dracula")
slug() { printf '%s' "$1" | tr '[:upper:] ' '[:lower:]-' | tr -cd 'a-z0-9-'; }

# ── GUI ──────────────────────────────────────────────────────────────────────
if [ "$(uname)" = Darwin ]; then
    bash "$root/examples/capture/gui.sh" "$shots" "${THEMES[@]}"
    cp "$shots/gui-tokyo-night.png" "$gen/gui.png"
    frames=(); for t in "${THEMES[@]}"; do frames+=("$shots/gui-$(slug "$t").png"); done
    bash "$root/examples/capture/gif.sh" "$gen/gui.gif" 1.8 "${frames[@]}"
else
    echo "skipping the GUI (macOS only)"
fi

# ── Web ──────────────────────────────────────────────────────────────────────
bash "$root/examples/vhs/fixture.sh" >/dev/null
eval "$("$root/examples/vhs/fixture.sh" env)"
case "$(uname)" in
    Darwin) default_browser="/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" ;;
    *) default_browser="$(command -v chromium || command -v google-chrome || command -v brave-browser || true)" ;;
esac
BROWSER="${BROWSER:-$default_browser}"
[ -x "$BROWSER" ] || { echo "no Chromium-based browser found; set BROWSER=/path/to/it"; exit 1; }

port=3971; debug=9335
TOKENBURN_WEB_ASSETS="$root/target/debug/assets" "$root/target/debug/tokenburn-web" --port "$port" --no-open >/dev/null 2>&1 &
web=$!
prof="$(mktemp -d)"
"$BROWSER" --headless=new --no-first-run --remote-debugging-port="$debug" --user-data-dir="$prof" about:blank >/dev/null 2>&1 &
browser=$!
trap 'kill $web $browser 2>/dev/null || true; rm -rf "$shots" "$prof" 2>/dev/null || true' EXIT
sleep 8
DEBUG_PORT="$debug" node "$root/examples/capture/web.mjs" "$port" "$shots" "${THEMES[@]}"
cp "$shots/web-tokyo-night.png" "$gen/web.png"
frames=(); for t in "${THEMES[@]}"; do frames+=("$shots/web-$(slug "$t").png"); done
bash "$root/examples/capture/gif.sh" "$gen/web.gif" 1.8 "${frames[@]}"
