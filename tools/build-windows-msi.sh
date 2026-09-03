#!/usr/bin/env bash
# Build Verkstead-x86_64.msi: the one `verkstead` binary with the tray in it,
# the shim that opens the tray app from an icon, and the installer that puts
# the two of them somewhere and tells Windows where.
#
# **What is inside is the whole of Verkstead** — built with the `desktop`
# feature, which is that binary's default (ADR-0012, as amended). So the file a
# human starts from the Start menu and the file a session it spawned runs `ask`
# with are one file, which is the invariant the sandbox stands on. Each half is
# reached its own way: the shortcut names the shim, which supplies the `desktop`
# verb a shortcut has nowhere to write, and a session runs the binary beside it
# by path, saying a verb of its own.
#
# **Two files are why there is an installer at all.** The Linux and macOS
# artifacts wrap a directory — an AppImage its AppDir, a dmg its bundle — and a
# Windows download had nothing to wrap while it was one exe. A shim beside a
# binary is not one file, so what a human downloads is the msi, which is also
# where the `PATH` entry comes from: `verkstead ask` in a terminal opened after
# the install is the reason the installer knows about `PATH` at all.
# `tools/verkstead.wxs` beside this file is where all of that is said; this
# script builds what goes into it and runs the toolset over it.
#
# Run it on Windows, in the bash a runner has and a Git install brings. It takes
# everything from the working tree — the viewer from `web/dist`, the icon from
# `packaging/` — and leaves one file at a fixed path, printed at the end, which
# is what the release workflow uploads.
#
# x86_64 only, as the AppImage is: Windows on arm64 runs an x86_64 exe under
# emulation, so a machine without the native build is a machine that still runs
# this one.
set -euo pipefail
cd "$(dirname "$0")/.."

# The two packages, and the two files they leave. `verkstead-cli` is the whole
# app; `verkstead-desktop` is a library everywhere and on this platform also the
# shim, which is what its `shim` feature builds — see `crates/desktop/Cargo.toml`.
CLI_PACKAGE="verkstead-cli"
SHIM_PACKAGE="verkstead-desktop"
CLI="target/release/verkstead.exe"
SHIM="target/release/verkstead-desktop.exe"

ICON="packaging/net.tobico.Verkstead.ico"
SOURCE="tools/verkstead.wxs"

# Where everything this script makes goes, under the directory cargo already
# writes to, so that one `.gitignore` line covers it and `cargo clean` takes it
# away.
WORK="target/windows"
OBJECT="$WORK/verkstead.wixobj"
MSI="$WORK/Verkstead-x86_64.msi"

say() { printf '%s\n' "$*"; }
die() {
  printf '%s\n' "$*" >&2
  exit 1
}

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) ;;
  *) die "This builds a Windows installer and has to run on Windows." ;;
esac

# The viewer, which is embedded at compile time rather than read at runtime —
# see `crates/server/src/viewer.rs`. The embed is `allow_missing`, so a tree
# without it builds an app that serves a 503 and says nothing about why, which
# is exactly the artifact nobody wants to find out about after a release.
[ -f web/dist/index.html ] ||
  die "web/dist is empty: build the viewer first, with (cd web && pnpm build)."

# The icon, which is generated and committed rather than built here — see
# tools/generate-packaging.sh, which writes it from the same artwork the Linux
# launcher icons come from. The shim carries it as a resource and Apps &
# Features draws the installed entry with it, so it is wanted twice.
[ -f "$ICON" ] ||
  die "$ICON is not there: run tools/generate-packaging.sh."

# `candle` compiles the source and `light` links what it wrote. Both are the WiX
# toolset's, which is on the GitHub runner image with its `bin` on the machine
# `PATH`; a machine without it is one this cannot run on, said here rather than
# by whichever of the two was reached for first.
for tool in cargo candle.exe light.exe; do
  command -v "$tool" > /dev/null ||
    die "$tool is needed to build the msi and is not on the PATH."
done

# The viewer is compiled in through `include_bytes!`, one file at a time, and
# cargo rebuilds when one of them changes — but a `web/dist` that was empty at
# the last build left nothing to notice, so a binary built before the viewer was
# would be reused with no viewer in it, and this script would wrap it. Touching
# the file that declares the embed is what says otherwise.
touch crates/server/src/viewer.rs

# The two packages by name rather than the whole workspace, for the reason
# `nix/verkstead-source.nix` gives: `verkstead-render`'s own default features
# turn on the TypeScript emitter, which is a test's business. Nothing here turns
# the `desktop` feature off, unlike every headless build of the same package —
# this is the artifact that wants the tray half, and it wants the other half in
# the same file. And `verkstead-desktop/shim` is what builds the exe beside it:
# the crate is a library on every platform and that feature is how Cargo is told
# a binary belongs to one of them.
say "Building $CLI and the shim beside it…"
cargo build --release --locked \
  --package "$CLI_PACKAGE" --package "$SHIM_PACKAGE" \
  --features "$SHIM_PACKAGE/shim"

for built in "$CLI" "$SHIM"; do
  [ -f "$built" ] || die "cargo left no $built."
done

# The version the installer claims, asked of cargo rather than read off
# `Cargo.toml` — `cargo pkgid` answers for the package that was just built, so
# the number in Apps & Features is the number the binary was built at.
VERSION="$(cargo pkgid --package "$CLI_PACKAGE")"
VERSION="${VERSION##*@}"

# **Three numbers and nothing else**, which is the one place an msi cannot say
# what the tag says: Windows Installer reads a product version as major, minor
# and build, and ignores everything after the third field — it will not accept a
# `0.1.0-rc.1` at all. So a pre-release ships as its own release number, and the
# `AllowSameVersionUpgrades` in the source is what makes the second rc replace
# the first rather than stand beside it.
PRODUCT_VERSION="${VERSION%%-*}"
say "Version $VERSION, which the installer carries as ${PRODUCT_VERSION}…"

# Windows paths for the Windows programs. `candle` and `light` are native
# programs and take none of the `/c/…` this shell speaks, and the working tree
# is reached through a path with no drive letter in it here — so every path
# handed over below goes through `cygpath -w` rather than being written twice.
rm -rf "$WORK"
mkdir -p "$WORK"

say "Compiling ${SOURCE}…"
candle.exe -nologo -arch x64 \
  -dVersion="$PRODUCT_VERSION" \
  -dCli="$(cygpath -w "$CLI")" \
  -dShim="$(cygpath -w "$SHIM")" \
  -dIcon="$(cygpath -w "$ICON")" \
  -out "$(cygpath -w "$OBJECT")" \
  "$(cygpath -w "$SOURCE")"

# No extensions and no `-sval`: nothing in the source reaches outside the
# toolset's own schema, and the validation `light` runs by default is what
# checks the per-user shape of the package — the profile directories, the
# registry key paths under them and the shortcut that names one. A package that
# would install wrongly fails here rather than on a downloader's machine.
say "Linking ${MSI}…"
light.exe -nologo \
  -out "$(cygpath -w "$MSI")" \
  "$(cygpath -w "$OBJECT")"

[ -s "$MSI" ] || die "The toolset wrote no $MSI."

say ""
say "$MSI"
