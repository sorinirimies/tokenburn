#!/usr/bin/env bash
set -euo pipefail
# Usage: ./scripts/ci/package_archive.sh <version> <target> [suffix]
#
# Builds one portable archive per target with every binary that was built:
#
#   tokenburn-<version>-<target>/
#     tokenburn[.exe]  tokenburn-tui[.exe]  tokenburn-gui[.exe]  tokenburn-web[.exe]
#     assets/            (web runtime bundle — next to the binary, found automatically)
#     README.md  LICENSE
#
# .tar.gz on Linux/macOS, .zip on Windows (suffix ".exe"). A .sha256 file is
# written next to it. The static musl Linux targets are the "runs on any
# distro" option for the CLI/TUI/web (no GUI there).
VERSION="$1"
TARGET="$2"
SUFFIX="${3:-}"
DIST="dist"
NAME="tokenburn-${VERSION}-${TARGET}"
STAGE="$DIST/$NAME"
BIN_DIR="target/$TARGET/release"

rm -rf "$STAGE"
mkdir -p "$STAGE"

found=0
for BIN in tokenburn tokenburn-tui tokenburn-gui tokenburn-web; do
    SRC="$BIN_DIR/$BIN$SUFFIX"
    if [ -f "$SRC" ]; then
        cp "$SRC" "$STAGE/$BIN$SUFFIX"
        chmod +x "$STAGE/$BIN$SUFFIX"
        found=$((found + 1))
    fi
done
if [ "$found" -eq 0 ]; then
    echo "❌ no binaries found in $BIN_DIR"
    exit 1
fi

if [ -f "$DIST/web-assets/manifest.toml" ]; then
    cp -R "$DIST/web-assets" "$STAGE/assets"
fi
cp README.md LICENSE "$STAGE/"

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi
}

if [ -n "$SUFFIX" ]; then
    OUT="$NAME.zip"
    if command -v zip >/dev/null 2>&1; then
        (cd "$DIST" && zip -qr "$OUT" "$NAME")
    elif command -v 7z >/dev/null 2>&1; then
        (cd "$DIST" && 7z a -tzip -bso0 "$OUT" "$NAME")
    else
        (cd "$DIST" && powershell -NoProfile -Command "Compress-Archive -Path '$NAME' -DestinationPath '$OUT' -Force")
    fi
else
    OUT="$NAME.tar.gz"
    (cd "$DIST" && tar -czf "$OUT" "$NAME")
fi

(cd "$DIST" && sha256 "$OUT" > "$OUT.sha256")
rm -rf "$STAGE"
echo "✅ $DIST/$OUT ($found binaries$( [ -f "$DIST/web-assets/manifest.toml" ] && echo ' + web assets' ))"
