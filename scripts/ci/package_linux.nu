#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# TokenBurn — Linux packaging: .deb and .rpm for CLI, TUI, GUI and web
# ──────────────────────────────────────────────────────────────────────────────
# Usage:
#   nu scripts/ci/package_linux.nu <version> <target>
#
# Requires: dpkg-deb (for .deb), alien (for .deb -> .rpm conversion).
# Tools are installed in the CI job that calls this script.
#
# Packages (one per binary):
#   tokenburn        /usr/bin/tokenburn                      (clap CLI)
#   tokenburn-tui    /usr/bin/tokenburn-tui + .desktop       (terminal UI)
#   tokenburn-gui    /usr/bin/tokenburn-gui + .desktop, icons, AppStream
#   tokenburn-web    /usr/bin/tokenburn-web + browser runtime assets
#                    in /usr/share/tokenburn-web/assets (when dist/web-assets
#                    exists — see scripts/ci/bundle_web.sh)
#
# The GUI links nothing against X11/Wayland/GL at build time: winit and wgpu
# `dlopen` them at runtime, so one binary serves X11 *and* Wayland sessions
# and every distro. The package therefore only hard-depends on libxkbcommon
# and merely *recommends* the display-server libraries.
# ──────────────────────────────────────────────────────────────────────────────

# Build the argument list for the `alien` invocation used to convert one
# already-built .deb into a .rpm. Extracted into its own pure function so the
# exact shape of the args can be asserted directly in tests.
#
# .rpm packages were previously built directly with `rpmbuild` from a
# hand-written .spec file, which kept failing on cross-architecture builds
# (rpmbuild validates `BuildArch:` against the host's compatible-architecture
# table and aarch64 <-> x86_64 are never compatible there; no flag bypasses it).
# `alien` sidesteps the problem entirely: it derives the .rpm's metadata
# directly from the already-built, already-correctly-tagged .deb's control file
# and performs no architecture feasibility check — it's just repackaging.
export def alien-args [deb_path: string]: nothing -> list<string> {
    ["--to-rpm" "--scripts" $deb_path]
}

# Whether a file name looks like a Linux package artifact (.deb or .rpm).
export def is-linux-package-file [name: string]: nothing -> bool {
    ($name | str ends-with ".deb") or ($name | str ends-with ".rpm")
}

# DEBIAN/control contents. Pure (no I/O) so the exact text can be asserted.
# `depends` / `recommends` are comma-separated lists, empty to omit the field.
export def deb-control [
    name: string
    version: string
    arch: string
    summary: string
    long: string
    depends: string
    recommends: string
]: nothing -> string {
    let dep_line = if ($depends | is-empty) { "" } else { $"Depends: ($depends)\n" }
    let rec_line = if ($recommends | is-empty) { "" } else { $"Recommends: ($recommends)\n" }
    $"Package: ($name)
Version: ($version)
Architecture: ($arch)
Maintainer: Sorin Irimies <sorinirimies@gmail.com>
Section: utils
Priority: optional
Description: ($summary)
 ($long)
Homepage: https://github.com/sorinirimies/tokenburn
($dep_line)($rec_line)"
}

# Control file for the desktop GUI package.
#
# NOTE: the literal "\(Elm Architecture\)" must stay escaped — an unescaped
# `(...)` inside a Nu `$"..."` string is evaluated as a command call. This is a
# regression guard (see the tests).
export def gui-deb-control [version: string, arch: string]: nothing -> string {
    (deb-control "tokenburn-gui" $version $arch
        "TokenBurn — desktop GUI AI token usage report written in Rust"
        "A mouse-driven desktop GUI for AI token/cost usage, built on Iced \(Elm Architecture\).\n Runs on X11 and Wayland."
        "libxkbcommon0"
        "libwayland-client0, libx11-6, libvulkan1 | libgl1, fontconfig, fonts-dejavu-core")
}

# Install-time layout of one package, as `{src, dst}` pairs relative to the
# repo root / package root. Pure so it can be unit tested.
export def package-files [name: string, target: string, has_web_assets: bool]: nothing -> list<record<src: string, dst: string>> {
    let bin = {|b| {src: $"target/($target)/release/($b)", dst: $"usr/bin/($b)"} }
    match $name {
        "tokenburn" => [(do $bin "tokenburn")]
        "tokenburn-tui" => [
            (do $bin "tokenburn-tui")
            {src: "packaging/linux/tokenburn-tui.desktop", dst: "usr/share/applications/tokenburn-tui.desktop"}
        ]
        "tokenburn-gui" => (
            [
                (do $bin "tokenburn-gui")
                {src: "packaging/linux/tokenburn-gui.desktop", dst: "usr/share/applications/tokenburn-gui.desktop"}
                {src: "packaging/linux/tokenburn-gui.metainfo.xml", dst: "usr/share/metainfo/tokenburn-gui.metainfo.xml"}
                {src: "assets/icons/tokenburn.svg", dst: "usr/share/icons/hicolor/scalable/apps/tokenburn.svg"}
            ] ++ (
                [16 22 24 32 48 64 96 128 256 512] | each {|n|
                    {src: $"assets/icons/tokenburn-($n).png", dst: $"usr/share/icons/hicolor/($n)x($n)/apps/tokenburn.png"}
                }
            )
        )
        "tokenburn-web" => (
            [(do $bin "tokenburn-web")]
            ++ (if $has_web_assets { [{src: "dist/web-assets", dst: "usr/share/tokenburn-web/assets"}] } else { [] })
        )
        _ => (error make {msg: $"unknown package: ($name)"})
    }
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
    # A plain interpolated string is ambiguous across nu versions -- some treat
    # it as a literal filename lookup ("DoNotExpand") instead of a glob.
    let pattern = ($out_dir + "/*.rpm" | into glob)
    (ls $pattern | first).name
}

# Stage + build one .deb and its .rpm. Returns nothing; prints a line per file.
def build-package [
    name: string
    version: string
    arch: string
    rpm_arch: string
    target: string
    control: string
    has_web_assets: bool
] {
    let dist_dir = "dist"
    let root = $"($dist_dir)/deb-($name)"
    rm -rf $root
    mkdir $"($root)/DEBIAN"

    for f in (package-files $name $target $has_web_assets) {
        let dst = $"($root)/($f.dst)"
        mkdir ($dst | path dirname)
        if ($f.src | path type) == "dir" {
            cp -r $f.src $dst
        } else {
            cp $f.src $dst
        }
    }
    chmod 755 $"($root)/usr/bin/($name)"

    mkdir $"($root)/usr/share/doc/($name)"
    $"($name) ($version)
Copyright Sorin Irimies
MIT License — see /usr/share/common-licenses/MIT
" | save -f $"($root)/usr/share/doc/($name)/copyright"

    $control | save -f $"($root)/DEBIAN/control"

    let deb_path = $"($dist_dir)/($name)_($version)_($arch).deb"
    run-external "dpkg-deb" "--root-owner-group" "--build" $root $deb_path
    print $"✅ Built ($name)_($version)_($arch).deb"

    # Each conversion uses its own scratch subdirectory so a leftover .rpm from
    # one conversion can never be mistaken for another's output.
    let rpm_file = (alien-deb-to-rpm $deb_path $"($dist_dir)/alien-rpm/($name)")
    cp $rpm_file $"($dist_dir)/($name)-($version)-($rpm_arch).rpm"
    print $"✅ Built ($name)-($version)-($rpm_arch).rpm"
}

def main [
    version: string   # e.g. 0.7.7
    target: string    # e.g. x86_64-unknown-linux-gnu
] {
    let arch = if ($target | str contains "aarch64") { "arm64" } else { "amd64" }
    let rpm_arch = if ($target | str contains "aarch64") { "aarch64" } else { "x86_64" }
    let has_web_assets = ("dist/web-assets" | path exists)

    mkdir dist

    build-package "tokenburn" $version $arch $rpm_arch $target (deb-control "tokenburn" $version $arch
        "TokenBurn — local AI token/cost usage CLI written in Rust"
        "Reads the local pi and Zed logs and prints token and cost usage per month, day or hour. No network access."
        "" "") $has_web_assets

    build-package "tokenburn-tui" $version $arch $rpm_arch $target (deb-control "tokenburn-tui" $version $arch
        "TokenBurn TUI — terminal AI token usage report written in Rust"
        "A keyboard-driven terminal UI for AI token/cost usage, built on Ratatui."
        "" "") $has_web_assets

    build-package "tokenburn-gui" $version $arch $rpm_arch $target (gui-deb-control $version $arch) $has_web_assets

    build-package "tokenburn-web" $version $arch $rpm_arch $target (deb-control "tokenburn-web" $version $arch
        "TokenBurn web — local AI token usage dashboard written in Rust"
        "A local web dashboard for AI token/cost usage, built on Topcoat. Binds 127.0.0.1 by default."
        "" "") $has_web_assets

    print ""
    print "📦 Linux packages:"
    ls dist | where { |it| is-linux-package-file ($it.name | path basename) } | select name size | print
}
