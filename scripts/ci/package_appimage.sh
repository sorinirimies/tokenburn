#!/usr/bin/env bash
set -euo pipefail
# Usage: ./scripts/ci/package_appimage.sh <version> <target>
#
# Builds one AppImage per interactive app (TUI, GUI). An AppImage runs on any
# glibc-based distro without installing anything. The GUI dlopen()s
# X11/Wayland/GL at runtime, so the same AppImage works in X11 *and* Wayland
# sessions; only the base glibc of the build machine matters (build on the
# oldest runner you can).
VERSION="$1"
TARGET="$2"
DIST="dist"
ARCH=$(uname -m)
ICON_SRC="assets/icons/tokenburn.png"

if [ ! -f "$ICON_SRC" ]; then
    echo "❌ $ICON_SRC missing — run: python3 scripts/gen_icons.py"
    exit 1
fi

mkdir -p "$DIST"

for APP in tokenburn-tui tokenburn-gui; do
    APPDIR="$DIST/AppDir-$APP"
    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
             "$APPDIR/usr/share/icons/hicolor/256x256/apps"

    BIN_SRC="target/$TARGET/release/$APP"
    if [ ! -f "$BIN_SRC" ]; then
        echo "⚠️  $BIN_SRC not found — skipping AppImage for $APP"
        continue
    fi

    cp "$BIN_SRC" "$APPDIR/usr/bin/$APP"
    chmod +x "$APPDIR/usr/bin/$APP"

    # Desktop entry: reuse the packaged one (single source of truth), pointing
    # the icon at the shared `tokenburn` icon (the TUI entry normally uses a
    # theme icon, which doesn't exist inside an AppDir).
    sed -e 's|^Icon=.*|Icon=tokenburn|' "packaging/linux/$APP.desktop" > "$APPDIR/$APP.desktop"
    cp "$APPDIR/$APP.desktop" "$APPDIR/usr/share/applications/"

    cp "$ICON_SRC" "$APPDIR/tokenburn.png"
    cp "$ICON_SRC" "$APPDIR/usr/share/icons/hicolor/256x256/apps/tokenburn.png"
    cp assets/icons/tokenburn.svg "$APPDIR/tokenburn.svg" 2>/dev/null || true

    cat > "$APPDIR/AppRun" <<APPRUN
#!/bin/sh
exec "\$(dirname "\$(readlink -f "\$0")")/usr/bin/$APP" "\$@"
APPRUN
    chmod +x "$APPDIR/AppRun"

    OUTPUT="$DIST/${APP}-${VERSION}-${ARCH}.AppImage"
    # GitHub-hosted Ubuntu runners don't have FUSE available, and appimagetool
    # is itself an AppImage that needs FUSE to mount and run by default.
    # APPIMAGE_EXTRACT_AND_RUN makes it extract itself to a temp dir instead.
    ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 appimagetool "$APPDIR" "$OUTPUT"
    echo "✅ Built $OUTPUT"
done
