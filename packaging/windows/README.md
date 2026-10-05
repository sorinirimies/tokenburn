# Windows Packaging

## NSIS Installer

The `installer.nsi` script builds a standard Windows installer (`.exe`) using
[NSIS](https://nsis.sourceforge.io/).

### Building locally

1. Install NSIS: https://nsis.sourceforge.io/Download
2. Build the binaries:
   ```
   cargo build --release -p tokenburn -p tokenburn-gui -p tokenburn-web -p tokenburn-tui --target x86_64-pc-windows-msvc
   ```
3. Replace `@VERSION@` and `@REPO_ROOT_ABS@` in the `.nsi` file (see
   `scripts/ci/package_windows.sh` for how CI does this)
4. Run: `makensis packaging\windows\installer.nsi`

The installer will be created at `dist\tokenburn-<version>-windows-x86_64-setup.exe`.

### Path resolution (`!cd`)

NSIS resolves every relative path used by `File`, `LicenseData`,
`MUI_ICON`/`MUI_UNICON`, and similar commands relative to the *compiled
script's own directory*, not the process's working directory. In CI this
script is compiled from a versioned copy at `dist\installer_versioned.nsi`
(see `scripts/ci/package_windows.sh`), so left unhandled, relative paths
like `LICENSE` or `target\...\release\tokenburn.exe` would be looked up
under `dist\`, where they don't exist.

`installer.nsi` opens with `!cd "@REPO_ROOT_ABS@"`, which
`package_windows.sh` substitutes with the actual repository root
(resolved via Git Bash's `pwd -W`) before invoking `makensis`. This
re-anchors every relative path in the script to the repo root in one
place, so `LICENSE`, `packaging\windows\tokenburn.ico`, and the `File`
commands all resolve exactly as if the script were compiled in place.

## Icon

`packaging\windows\tokenburn.ico` is committed (classic BMP entries at
16/24/32/48 px, which NSIS' legacy icon loader accepts — it rejects
PNG-compressed entries). Regenerate every icon asset with
`python3 scripts/gen_icons.py`. `package_windows.sh` fails loudly if the file
is missing rather than shipping a placeholder.
