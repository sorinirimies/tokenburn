#!/usr/bin/env bash
# Stitch screenshots into a looping GIF (ffmpeg, two-pass palette for clean colours).
#
#   examples/capture/gif.sh <out.gif> <seconds-per-frame> <png> [png ...]
set -euo pipefail
out="${1:?usage: gif.sh <out.gif> <seconds-per-frame> <png>...}"; secs="$2"; shift 2
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
i=0
for f in "$@"; do i=$((i + 1)); cp "$f" "$tmp/$(printf '%03d' "$i").png"; done
ffmpeg -loglevel error -y -framerate "$(awk "BEGIN{print 1/$secs}")" -i "$tmp/%03d.png" \
    -vf "scale=${GIF_WIDTH:-1100}:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4" \
    -loop 0 "$out"
echo "wrote $out"
