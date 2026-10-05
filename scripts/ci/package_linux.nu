#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# TokenBurn — Linux packaging: .deb, .rpm, AppImage
# ──────────────────────────────────────────────────────────────────────────────
# Usage:
#   nu scripts/ci/package_linux.nu <version> <target>
#
# Requires: dpkg-deb (for .deb), alien (for .deb -> .rpm conversion),
# appimagetool (for AppImage). Tools are installed in the CI job that calls
# this script.
# ──────────────────────────────────────────────────────────────────────────────

# Build the argument list for the `alien` invocation used to convert one
# already-built .deb into a .rpm. Extracted into its own pure function so the
# exact shape of the args can be asserted directly in tests.
#
# .rpm packages were previously built directly with `rpmbuild` from a
# hand-written .spec file (see git history for `rpmbuild-args` /
# `gui-rpm-spec`), which went through two failed fix attempts:
#   1. Missing `--target` made cross-architecture builds (e.g. an aarch64
#      .rpm on an x86_64 CI runner) fail with "No compatible architectures
#      found for build".
#   2. Adding `--target <arch>-linux` was the wrong fix: it made rpmbuild
#      validate cross-build *feasibility* against the host's
#      compatible-architecture table, which Ubuntu's rpm package has no
#      aarch64<->x86_64 entries for -- so it failed with the *exact same*
#      error message (CI run 30751058486).
#   3. Removing --target again didn't help either (CI run 30752224652):
#      rpmbuild validates `BuildArch:` in the .spec against the host's
#      compatible-architecture table regardless of --target, and aarch64
#      and x86_64 are simply never considered compatible in that table on
#      this rpm build. There is no rpmbuild flag that bypasses this for a
#      genuine foreign architecture on this platform.
#
# `alien` sidesteps the problem entirely: it derives the .rpm's metadata
# (name, version, architecture, description, etc.) directly from the
# already-built, already-correctly-tagged .deb's control file, and doesn't
# perform any host/target architecture *compilation*-feasibility check --
# it's just repackaging existing files, which is all we ever needed here.
export def alien-args [deb_path: string]: nothing -> list<string> {
    ["--to-rpm" "--scripts" $deb_path]
}

# Whether a file name looks like a Linux package artifact (.deb or .rpm).
# Extracted so the glob/regex used by the final packaging summary can be
# unit tested directly instead of only being exercised as a side effect of
# `ls | where ...` at the end of a full packaging run.
export def is-linux-package-file [name: string]: nothing -> bool {
    ($name | str ends-with ".deb") or ($name | str ends-with ".rpm")
}

# Build the DEBIAN/control file contents for the desktop GUI .deb package.
#
# Extracted into its own pure function (no side effects, no external
# commands) so its exact text can be asserted in tests. This is a regression
# guard for a bug where the literal parenthesised text "(Elm Architecture)"
# inside a `$"..."` interpolated string was parsed by Nu as a *nested
# command call* (`Elm Architecture`) rather than literal text, since any
# unescaped `(...)` inside a Nu interpolated string is evaluated as an
# expression. That bug could not be caught by `nu-check` / parsing alone —
# it only failed at runtime, when the packaging step actually reached this
# code path.
export def gui-deb-control [version: string, arch: string]: nothing -> string {
    $"Package: tokenburn-gui
Version: ($version)
Architecture: ($arch)
Maintainer: Sorin Irimies <sorinirimies@gmail.com>
Description: TokenBurn — desktop GUI AI token usage report written in Rust
 A mouse-driven desktop GUI for AI token/cost usage, built on Iced \(Elm Architecture\).
Homepage: https://github.com/sorinirimies/tokenburn
Depends: libxkbcommon0, libwayland-client0, libgl1
"
}

# Convert an already-built .deb into a .rpm using `alien`, and return the
# path to the produced .rpm file.
#
# `alien` writes its output to the current working directory rather than
# accepting an output-directory flag, so this runs it from a dedicated
# scratch directory and restores the previous working directory afterward.
export def alien-deb-to-rpm [
    deb_path: string   # e.g. dist/tokenburn-tui_1.1.6_arm64.deb
    out_dir: string     # e.g. dist/alien-rpm
]: nothing -> string {
    mkdir $out_dir
    let deb_abs = ($deb_path | path expand)
    let prev_dir = (pwd)
    cd $out_dir
    run-external "alien" ...(alien-args $deb_abs)
    cd $prev_dir
    # `into glob` forces glob expansion of this dynamically-built pattern.
    # A plain interpolated string (e.g. `ls $"(...)/*.rpm"`) is ambiguous
    # across nu versions -- some treat it as a literal filename lookup
    # ("DoNotExpand") instead of a glob, which would fail here even though
    # a matching file exists.
    let pattern = ($out_dir + "/*.rpm" | into glob)
    (ls $pattern | first).name
}

def main [
    version: string   # e.g. 0.7.7
    target: string    # e.g. x86_64-unknown-linux-gnu
] {
    let arch = if ($target | str contains "aarch64") { "arm64" } else { "amd64" }
    let rpm_arch = if ($target | str contains "aarch64") { "aarch64" } else { "x86_64" }
    let dist_dir = "dist"

    mkdir $dist_dir

    # ── .deb for tokenburn-tui ────────────────────────────────────────────────
    let tui_deb_root = $"($dist_dir)/deb-tui"
    mkdir $"($tui_deb_root)/DEBIAN"
    mkdir $"($tui_deb_root)/usr/bin"
    mkdir $"($tui_deb_root)/usr/share/doc/tokenburn-tui"

    cp $"target/($target)/release/tokenburn-tui" $"($tui_deb_root)/usr/bin/tokenburn-tui"

    $"Package: tokenburn-tui
Version: ($version)
Architecture: ($arch)
Maintainer: Sorin Irimies <sorinirimies@gmail.com>
Description: TokenBurn TUI — terminal AI token usage report written in Rust
 A keyboard-driven terminal UI for AI token/cost usage, built on Ratatui.
Homepage: https://github.com/sorinirimies/tokenburn
" | save -f $"($tui_deb_root)/DEBIAN/control"

    $"TokenBurn TUI ($version)
Copyright 2024 Sorin Irimies
MIT License — see /usr/share/common-licenses/MIT
" | save -f $"($tui_deb_root)/usr/share/doc/tokenburn-tui/copyright"

    let tui_deb_path = $"($dist_dir)/tokenburn-tui_($version)_($arch).deb"
    run-external "dpkg-deb" "--build" $tui_deb_root $tui_deb_path
    print $"✅ Built tokenburn-tui_($version)_($arch).deb"

    # ── .deb for tokenburn (GUI) ──────────────────────────────────────────────
    let gui_deb_root = $"($dist_dir)/deb-gui"
    mkdir $"($gui_deb_root)/DEBIAN"
    mkdir $"($gui_deb_root)/usr/bin"
    mkdir $"($gui_deb_root)/usr/share/doc/tokenburn-gui"

    cp $"target/($target)/release/tokenburn-gui" $"($gui_deb_root)/usr/bin/tokenburn-gui"

    (gui-deb-control $version $arch) | save -f $"($gui_deb_root)/DEBIAN/control"

    $"TokenBurn ($version)
Copyright 2024 Sorin Irimies
MIT License — see /usr/share/common-licenses/MIT
" | save -f $"($gui_deb_root)/usr/share/doc/tokenburn-gui/copyright"

    let gui_deb_path = $"($dist_dir)/tokenburn-gui_($version)_($arch).deb"
    run-external "dpkg-deb" "--build" $gui_deb_root $gui_deb_path
    print $"✅ Built tokenburn-gui_($version)_($arch).deb"

    # ── .rpm for tokenburn-tui and tokenburn (via alien, from the .deb above) ──
    # Each conversion uses its own scratch subdirectory so a leftover .rpm
    # from one conversion can never be mistaken for the other's output by
    # the "pick the only/first .rpm in this directory" lookup in
    # `alien-deb-to-rpm`.
    let tui_rpm_file = (alien-deb-to-rpm $tui_deb_path $"($dist_dir)/alien-rpm/tui")
    cp $tui_rpm_file $"($dist_dir)/tokenburn-tui-($version)-($rpm_arch).rpm"
    print $"✅ Built tokenburn-tui-($version)-($rpm_arch).rpm"

    let gui_rpm_file = (alien-deb-to-rpm $gui_deb_path $"($dist_dir)/alien-rpm/gui")
    cp $gui_rpm_file $"($dist_dir)/tokenburn-gui-($version)-($rpm_arch).rpm"
    print $"✅ Built tokenburn-gui-($version)-($rpm_arch).rpm"

    print ""
    print "📦 Linux packages:"
    ls $dist_dir | where { |it| is-linux-package-file ($it.name | path basename) } | select name size | print
}
