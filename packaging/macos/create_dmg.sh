#!/usr/bin/env bash
set -euo pipefail
# Usage: ./packaging/macos/create_dmg.sh <version>
#
# Creates a macOS DMG containing
#   TokenBurn.app            the desktop GUI as a proper app bundle (with icon)
#   Command Line Tools/      tokenburn (CLI), tokenburn-tui, tokenburn-web (+ assets/)
#
# Expects (universal or per-arch) release binaries under target/<triple>/release/
# and, optionally, the web runtime bundle in dist/web-assets (see
# scripts/ci/bundle_web.sh). Requires create-dmg (brew install create-dmg) for
# the styled window; falls back to a plain hdiutil image.

VERSION="$1"
DIST="dist"
APP_NAME="TokenBurn"
ROOT="${DIST}/dmg-root"
BUNDLE="${ROOT}/${APP_NAME}.app"
CLI_DIR="${ROOT}/Command Line Tools"
ICNS="packaging/macos/tokenburn.icns"

rm -rf "$ROOT"
mkdir -p "$DIST" "${BUNDLE}/Contents/MacOS" "${BUNDLE}/Contents/Resources" "$CLI_DIR"

# Pick the universal binary if present, else whichever single-arch build exists.
pick() {
    local bin="$1"
    for triple in universal-apple-darwin aarch64-apple-darwin x86_64-apple-darwin; do
        if [ -f "target/${triple}/release/${bin}" ]; then
            echo "target/${triple}/release/${bin}"
            return 0
        fi
    done
    return 1
}

# ── GUI app bundle ───────────────────────────────────────────────────────────
GUI_BIN="$(pick tokenburn-gui)" || { echo "❌ tokenburn-gui binary not found"; exit 1; }
cp "$GUI_BIN" "${BUNDLE}/Contents/MacOS/tokenburn-gui"
chmod +x "${BUNDLE}/Contents/MacOS/tokenburn-gui"

if [ -f "$ICNS" ]; then
    cp "$ICNS" "${BUNDLE}/Contents/Resources/tokenburn.icns"
else
    echo "⚠️  $ICNS missing — the app will have a generic icon (run: python3 scripts/gen_icons.py)"
fi

cat > "${BUNDLE}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>tokenburn-gui</string>
    <key>CFBundleIdentifier</key>
    <string>com.sorinirimies.tokenburn</string>
    <key>CFBundleName</key>
    <string>TokenBurn</string>
    <key>CFBundleDisplayName</key>
    <string>TokenBurn</string>
    <key>CFBundleIconFile</key>
    <string>tokenburn</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

# Ad-hoc sign so the bundle has a valid (if unidentified) signature; Apple
# Silicon refuses to run unsigned arm64 code. Users still need to right-click →
# Open the first time unless the app is notarised with a Developer ID.
if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "${BUNDLE}" || echo "⚠️  ad-hoc codesign failed (continuing)"
fi
echo "✅ Created ${APP_NAME}.app bundle"

# ── Command-line tools ───────────────────────────────────────────────────────
for BIN in tokenburn tokenburn-tui tokenburn-web; do
    if SRC="$(pick "$BIN")"; then
        cp "$SRC" "${CLI_DIR}/${BIN}"
        chmod +x "${CLI_DIR}/${BIN}"
    else
        echo "⚠️  ${BIN} not found — not included in the DMG"
    fi
done
if [ -f "${DIST}/web-assets/manifest.toml" ]; then
    cp -R "${DIST}/web-assets" "${CLI_DIR}/assets"
fi
cat > "${CLI_DIR}/README.txt" <<TXT
Copy these files to a directory on your PATH, e.g.

    sudo cp tokenburn tokenburn-tui tokenburn-web /usr/local/bin/
    sudo cp -R assets /usr/local/bin/assets      # live web dashboard (optional)

If macOS blocks them as "from an unidentified developer":

    xattr -dr com.apple.quarantine tokenburn tokenburn-tui tokenburn-web
TXT

# ── Create DMG ───────────────────────────────────────────────────────────────
OUTPUT="${DIST}/tokenburn-${VERSION}-macos.dmg"
rm -f "$OUTPUT"

if command -v create-dmg &>/dev/null; then
    ARGS=(
        --volname "TokenBurn ${VERSION}"
        --window-pos 200 120
        --window-size 660 400
        --icon-size 128
        --icon "${APP_NAME}.app" 160 185
        --hide-extension "${APP_NAME}.app"
        --app-drop-link 500 185
        --no-internet-enable
    )
    [ -f "$ICNS" ] && ARGS+=(--volicon "$ICNS")
    create-dmg "${ARGS[@]}" "$OUTPUT" "$ROOT/"
else
    # Fallback: plain hdiutil DMG (no fancy layout) with an Applications link.
    ln -sf /Applications "${ROOT}/Applications"
    hdiutil create -volname "TokenBurn ${VERSION}" -srcfolder "$ROOT" -ov -format UDZO "$OUTPUT"
fi

if [ ! -s "$OUTPUT" ]; then
    echo "❌ DMG was not produced: $OUTPUT"
    exit 1
fi
rm -rf "$ROOT"
echo "✅ Built ${OUTPUT}"
