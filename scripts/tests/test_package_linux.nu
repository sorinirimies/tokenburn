#!/usr/bin/env nu
# ── TokenBurn · test_package_linux.nu ────────────────────────────────────────
# Tests for scripts/ci/package_linux.nu — Linux .deb/.rpm packaging helpers.
#
# These are regression tests for a CI failure where the two `rpmbuild`
# `run-external` calls were split across multiple lines with no valid Nu
# line-continuation, producing a `nu::parser::parse_mismatch` ("expected
# operator") instead of a normal rpmbuild invocation — plus:
# - a second, latent parse error from an unsupported `\.` escape inside a
#   double-quoted string used to filter package files,
# - a third, purely *runtime* bug (see gui-deb-control tests below) that
#   only surfaced once the first two were fixed and CI actually reached
#   that code path,
# - a fourth bug where cross-architecture rpm builds (e.g. building an
#   aarch64 .rpm on an x86_64 CI runner) failed with "No compatible
#   architectures found for build", which was (wrongly) "fixed" by adding
#   `--target <arch>-linux`,
# - a fifth bug: passing --target made things *worse* for genuine
#   cross-arch builds, since it tells rpmbuild to validate cross-build
#   feasibility against the host's compatible-architecture table (which
#   Ubuntu's rpm package doesn't have aarch64<->x86_64 entries for) --
#   producing the exact same error, and
# - a sixth bug: dropping --target again *still* didn't help (CI run
#   30752224652), because rpmbuild validates `BuildArch:` in the .spec
#   against that same host-compatibility table regardless of --target.
#   aarch64 and x86_64 are simply never compatible in that table on this
#   rpm build, with no flag to override it for a genuine foreign
#   architecture.
#
# The actual fix replaces rpmbuild + hand-written .spec files entirely
# with `alien`, which converts the already-built, already-correctly-tagged
# .deb directly into a .rpm without any host/target architecture
# compilation-feasibility check (it's pure repackaging of existing files,
# which is all this step ever needed to do).

use std/assert
use runner.nu *
use ../ci/package_linux.nu [alien-args, is-linux-package-file, gui-deb-control, deb-control, package-files]

# ── alien-args ───────────────────────────────────────────────────────────────

def "test alien-args: returns exactly three args" [] {
    let args = (alien-args "dist/tokenburn-tui_1.1.6_amd64.deb")
    assert equal ($args | length) 3
}

def "test alien-args: is --to-rpm --scripts <deb_path>" [] {
    let args = (alien-args "dist/tokenburn-tui_1.1.6_amd64.deb")
    assert equal ($args | get 0) "--to-rpm"
    assert equal ($args | get 1) "--scripts"
    assert equal ($args | get 2) "dist/tokenburn-tui_1.1.6_amd64.deb"
}

def "test alien-args: does not pass --target or a BuildArch-style flag" [] {
    # Regression guard: unlike the old rpmbuild-based approach, alien takes
    # no architecture flag at all -- it reads the architecture directly from
    # the .deb being converted, so there's no host-compatibility gate to
    # accidentally trip.
    let args = (alien-args "dist/tokenburn_2.0.0_arm64.deb")
    assert (not ($args | any { |it| $it == "--target" }))
    assert (not ($args | any { |it| $it | str contains "BuildArch" }))
}

def "test alien-args: works for the GUI deb path too" [] {
    let deb = "dist/tokenburn_9.9.9_arm64.deb"
    let args = (alien-args $deb)
    assert equal ($args | length) 3
    assert equal ($args | get 2) $deb
}

# ── is-linux-package-file ────────────────────────────────────────────────────

def "test is-linux-package-file: matches .deb files" [] {
    assert (is-linux-package-file "tokenburn-tui_1.2.3_amd64.deb")
}

def "test is-linux-package-file: matches .rpm files" [] {
    assert (is-linux-package-file "tokenburn-1.2.3-1.x86_64.rpm")
}

def "test is-linux-package-file: rejects unrelated files" [] {
    assert (not (is-linux-package-file "tokenburn-tui"))
    assert (not (is-linux-package-file "tokenburn.tar.gz"))
    assert (not (is-linux-package-file "notes.txt"))
}

def "test is-linux-package-file: does not match substrings mid-name" [] {
    # ".deb"/".rpm" must be the actual file extension, not just present
    # somewhere earlier in the name.
    assert (not (is-linux-package-file "deb-tui-staging-dir"))
    assert (not (is-linux-package-file "rpmbuild-notes.md"))
}

def "test is-linux-package-file: empty string is not a package" [] {
    assert (not (is-linux-package-file ""))
}

# ── gui-deb-control ───────────────────────────────────────────────────────────
# Regression tests for a second, purely *runtime* bug that the parse fix
# above could not catch: the literal parenthesised text "(Elm Architecture)"
# inside a `$"..."` interpolated string was parsed by Nu as a nested command
# call (`Elm Architecture`) instead of literal text, since any unescaped
# `(...)` inside a Nu interpolated string is evaluated as an expression. This
# only surfaced once the earlier parse-mismatch bug was fixed and CI actually
# reached this code path -- so these tests exercise the generated text
# directly rather than relying on a full end-to-end packaging run.

def "test gui-deb-control: contains the literal Elm Architecture parenthetical" [] {
    let control = (gui-deb-control "1.1.6" "amd64")
    assert ($control | str contains "(Elm Architecture)")
}

def "test gui-deb-control: interpolates version and architecture" [] {
    let control = (gui-deb-control "9.9.9" "arm64")
    assert ($control | str contains "Version: 9.9.9")
    assert ($control | str contains "Architecture: arm64")
}

def "test gui-deb-control: has the expected package name and maintainer" [] {
    let control = (gui-deb-control "1.0.0" "amd64")
    assert ($control | str contains "Package: tokenburn")
    assert ($control | str contains "Maintainer: Sorin Irimies")
}

def "test gui-deb-control: hard-depends only on libxkbcommon, display servers are recommended" [] {
    # The GUI dlopen()s X11/Wayland/GL at runtime, so one binary serves both
    # session types on every distro; those libs must not be hard dependencies.
    let control = (gui-deb-control "1.0.0" "amd64")
    assert ($control | str contains "Depends: libxkbcommon0\n")
    assert ($control | str contains "Recommends: libwayland-client0, libx11-6")
    assert (not ($control | str contains "Depends: libxkbcommon0, libwayland"))
}

# ── deb-control ───────────────────────────────────────────────────────────────

def "test deb-control: omits Depends and Recommends when empty" [] {
    let c = (deb-control "tokenburn" "1.0.0" "amd64" "sum" "long" "" "")
    assert (not ($c | str contains "Depends:"))
    assert (not ($c | str contains "Recommends:"))
    assert ($c | str contains "Package: tokenburn\n")
}

def "test deb-control: includes Depends when given" [] {
    let c = (deb-control "x" "1.0.0" "arm64" "sum" "long" "libfoo" "")
    assert ($c | str contains "Depends: libfoo")
    assert ($c | str contains "Architecture: arm64")
}

# ── package-files ─────────────────────────────────────────────────────────────

def "test package-files: every package installs its binary under usr/bin" [] {
    for name in ["tokenburn" "tokenburn-tui" "tokenburn-gui" "tokenburn-web"] {
        let files = (package-files $name "x86_64-unknown-linux-gnu" false)
        assert ($files | any {|f| $f.dst == $"usr/bin/($name)" })
    }
}

def "test package-files: only the GUI ships icons, desktop entry and metainfo" [] {
    # Icons must live in exactly one package: two debs owning the same path
    # would make dpkg refuse to install both.
    let gui = (package-files "tokenburn-gui" "t" false | get dst)
    assert ($gui | any {|d| $d == "usr/share/icons/hicolor/256x256/apps/tokenburn.png" })
    assert ($gui | any {|d| $d == "usr/share/applications/tokenburn-gui.desktop" })
    assert ($gui | any {|d| $d | str ends-with "tokenburn-gui.metainfo.xml" })
    for other in ["tokenburn" "tokenburn-tui" "tokenburn-web"] {
        let dsts = (package-files $other "t" false | get dst)
        assert (not ($dsts | any {|d| $d | str contains "icons/hicolor" }))
    }
}

def "test package-files: web assets only when the bundle exists" [] {
    let without = (package-files "tokenburn-web" "t" false | get dst)
    let with = (package-files "tokenburn-web" "t" true | get dst)
    assert (not ($without | any {|d| $d | str contains "tokenburn-web/assets" }))
    assert ($with | any {|d| $d == "usr/share/tokenburn-web/assets" })
}

def "test package-files: rejects unknown package" [] {
    let failed = (try { package-files "nope" "t" false; false } catch { true })
    assert $failed
}

# ── Parse-level regression guard ─────────────────────────────────────────────
# The original bug was a syntax error, not a logic error, so the most direct
# regression test is simply confirming the script still parses cleanly.

def "test package_linux.nu: parses without syntax errors" [] {
    assert (nu-check ($env.CURRENT_FILE | path dirname | path join ".." "ci" "package_linux.nu"))
}

def "test package_linux.nu: parses cleanly as a module too" [] {
    # Exercises the same `export def` parsing path used by `use ... [fn, ...]`
    # above, guarding against a regression that only manifests when the file
    # is imported as a module rather than run as a script.
    assert (nu-check --as-module ($env.CURRENT_FILE | path dirname | path join ".." "ci" "package_linux.nu"))
}

def "test package_linux.nu: uses into glob for the dynamic .rpm lookup pattern" [] {
    # Regression guard: `ls $"(...)/*.rpm"` (a plain interpolated string) is
    # ambiguous across nu versions -- some treat it as a literal filename
    # lookup ("DoNotExpand") rather than a glob, which fails even when a
    # matching file exists. `into glob` forces correct glob expansion.
    let text = (open --raw ($env.CURRENT_FILE | path dirname | path join ".." "ci" "package_linux.nu"))
    assert ($text | str contains "into glob")
}

def "test package_linux.nu: no longer invokes rpmbuild" [] {
    # Regression guard: rpmbuild + hand-written .spec files were replaced
    # entirely by alien (see module header for the full saga of why). Prose
    # comments are allowed to mention "rpmbuild" (to explain the history);
    # the actual external-command invocation must be gone.
    let text = (open --raw ($env.CURRENT_FILE | path dirname | path join ".." "ci" "package_linux.nu"))
    assert (not ($text | str contains "run-external \"rpmbuild\""))
    assert (not ($text | str contains "%install"))
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
