#!/usr/bin/env bash
# Screenshots of the desktop GUI at the three layout classes (macOS only: it brings
# the window to the front and uses screencapture). The window size comes from
# TOKENBURN_GUI_SIZE=WxH.
#
#   examples/capture/gui_responsive.sh <out-dir>
set -euo pipefail
[ "$(uname)" = Darwin ] || { echo "gui_responsive.sh needs macOS (screencapture)"; exit 1; }
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="${1:?usage: gui_responsive.sh <out-dir>}"
mkdir -p "$out"
bash "$root/examples/vhs/fixture.sh" >/dev/null
eval "$("$root/examples/vhs/fixture.sh" env)"

for entry in wide:1200x760 medium:800x800 narrow:400x820; do
    name="${entry%%:*}"; size="${entry#*:}"
    TOKENBURN_GUI_SIZE="$size" "$root/target/debug/tokenburn-gui" --daily --all --theme "Tokyo Night" >/dev/null 2>&1 &
    pid=$!
    sleep 6
    osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null 2>&1 || true
    sleep 1.5
    front=$(osascript -e 'tell application "System Events" to get unix id of first process whose frontmost is true')
    if [ "$front" != "$pid" ]; then kill "$pid" 2>/dev/null || true; echo "the GUI window is not in front; close other windows and retry"; exit 1; fi
    read -r x y w h < <(osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid) to get {position, size} of window 1" | tr -d ',' | awk '{print $1,$2,$3,$4}')
    screencapture -x -R "$x,$y,$w,$h" "$out/gui-$name.png"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    echo "wrote $out/gui-$name.png"
done
