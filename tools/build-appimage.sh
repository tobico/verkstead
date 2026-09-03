#!/usr/bin/env bash
# Build Verkstead-x86_64.AppImage: the one `verkstead` binary with the tray in
# it, the packaging assets and every library it is drawn over, in one file that
# runs on a machine which has none of them installed.
#
# **What is inside is the whole of Verkstead** — built with the `desktop`
# feature, which is that binary's default (ADR-0012, as amended). So the file a
# human double-clicks and the file a session it spawned runs `ask` with are one
# file, which is the invariant the sandbox stands on. Each half is reached its
# own way: `AppRun` supplies the `desktop` verb, because a human executing an
# AppImage passes no command line, and a session runs the binary inside the
# mounted image by path, saying a verb of its own.
#
# The released CLI is that same binary with the feature off — a static musl
# one, which needs none of this. This is the artifact that links system
# libraries — GTK3, and the appindicator the tray is drawn over (ADR-0012) —
# and making *it* static is not on: the toolkit is not built that way. So it
# stays dynamic and carries what it links, which is what an AppImage is for.
#
# Run it in the dev shell, or on a runner that has installed the two development
# packages `ci.yml` installs to build `crates/desktop` at all. It takes
# everything from the working tree — the viewer from `web/dist`, the assets from
# `packaging/` — and leaves one file at a fixed path, printed at the end, which
# is what the release workflow uploads.
#
# x86_64 only, per ADR-0012: an arm64 Linux user has the bare CLI.
set -euo pipefail
cd "$(dirname "$0")/.."

APP_ID="net.tobico.Verkstead"
# Two names where there was one: the package cargo is asked for, and the file
# it leaves — the tray app being a verb of the CLI's binary rather than a
# binary of its own.
PACKAGE="verkstead-cli"
BINARY="verkstead"

# Where everything this script makes goes, under the directory cargo already
# writes to, so that one `.gitignore` line covers it and `cargo clean` takes it
# away.
WORK="target/appimage"
APPDIR="$WORK/AppDir"
APPIMAGE="$WORK/Verkstead-x86_64.AppImage"

# What is left to the machine the AppImage lands on: the C runtime, and the
# loader that maps it. They are the one thing every glibc machine has and the
# one thing that cannot be replaced from inside a bundle — a process that had
# loaded two of them would have two of everything the C library keeps. They also
# settle the floor this artifact runs on, which is the glibc of the machine it
# was built on — which is why the release workflow does not build it on the
# runner, but in an older container image on top of one, and reads the symbols
# back afterwards to hold it there. Run here, the floor is the dev shell's.
#
# One list, read twice: by what is left out below, and by the check at the end
# that nothing else was.
RUNTIME_LIBS='^(ld-linux[^/]*|libc|libm|libdl|libpthread|librt|libresolv|libutil|libgcc_s)\.so'

# The AppImage runtime: the little static ELF that gets prepended to the
# filesystem image below, and that mounts it and runs AppRun when the human
# executes the file. It is the format itself, so there is nothing to build here
# — it is downloaded once and kept beside the output.
#
# Pinned to a dated release and checked by hash, the way everything else in this
# project is pinned: this is somebody else's binary ending up inside Verkstead's
# own artifact, and `continuous` — the tag the format's documentation points at
# — is a tag that moves under you.
RUNTIME_URL="https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64"
RUNTIME_SHA256="2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"
RUNTIME="$WORK/runtime-x86_64"

say() { printf '%s\n' "$*"; }
die() {
  printf '%s\n' "$*" >&2
  exit 1
}

# The viewer, which is embedded at compile time rather than read at runtime —
# see `crates/server/src/viewer.rs`. The embed is `allow_missing`, so a tree
# without it builds an app that serves a 503 and says nothing about why, which
# is exactly the artifact nobody wants to find out about after a release.
[ -f web/dist/index.html ] ||
  die "web/dist is empty: build the viewer first, with (cd web && pnpm build)."

for tool in cargo pkg-config ldd mksquashfs curl; do
  command -v "$tool" > /dev/null ||
    die "$tool is needed to build the AppImage and is not on the PATH."
done

# The viewer is compiled in through `include_bytes!`, one file at a time, and
# cargo rebuilds when one of them changes — but a `web/dist` that was empty at
# the last build left nothing to notice, so a binary built before the viewer was
# would be reused with no viewer in it, and this script would wrap it. Touching
# the file that declares the embed is what says otherwise, and it costs the two
# crates that carry the viewer rather than the whole workspace.
touch crates/server/src/viewer.rs

# The binary itself, for whatever this machine is, which by the header above is
# x86_64 Linux. No `--target`: the tray links system libraries, so a cross
# build would have nothing to link against — this leg is native or it is
# nothing, which is why the release workflow gives it a Linux runner of its own.
#
# The package by name rather than the whole workspace, for the reason
# `nix/verkstead-source.nix` gives: `verkstead-render`'s own default features
# turn on the TypeScript emitter, which is a test's business. And nothing here
# turns the `desktop` feature off, unlike every headless build of the same
# package — this is the artifact that wants the tray half.
say "Building $BINARY…"
cargo build --release --locked --package "$PACKAGE"

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/lib" "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons"
cp "target/release/$BINARY" "$APPDIR/usr/bin/$BINARY"

# What the loader resolves for a file, one path per line. It prints a resolved
# dependency two ways: the usual `name => /path`, and a bare path where what was
# named was an absolute one to begin with.
resolves() {
  ldd "$1" 2> /dev/null |
    awk '$2 == "=>" && $3 ~ /^\// { print $3 } $1 ~ /^\// { print $1 }'
}

# Every library a file needs, put where AppRun points the loader, and then the
# same again for whatever those libraries themselves need — `ldd` answers for
# the whole tree at once, so one call per starting point is the whole of it.
bundle() {
  local resolved name
  while read -r resolved; do
    name="${resolved##*/}"
    if [[ "$name" =~ $RUNTIME_LIBS ]]; then
      continue
    fi
    if [ ! -e "$APPDIR/usr/lib/$name" ]; then
      cp -L "$resolved" "$APPDIR/usr/lib/$name"
    fi
  done < <(resolves "$1")
}

say "Gathering what it links…"
bundle "$APPDIR/usr/bin/$BINARY"

# And the appindicator, which `ldd` knows nothing about: `libappindicator-sys`
# opens it by name at runtime rather than linking it, so it is absent from the
# binary's dependencies and would be absent from the bundle for the same reason
# — leaving an AppImage that runs everywhere and draws a tray icon nowhere.
#
# Found through pkg-config, which is how the same library is found at build
# time, so the dev shell and a runner both answer without either being written
# down here. The name is the first one the crate asks the loader for.
indicator="$(pkg-config --variable=libdir ayatana-appindicator3-0.1)/libayatana-appindicator3.so.1"
[ -e "$indicator" ] ||
  die "$indicator is not there: the tray would have nothing to draw itself on."
cp -L "$indicator" "$APPDIR/usr/lib/libayatana-appindicator3.so.1"
bundle "$indicator"

# Writable again, so that copies out of a read-only prefix — a nix store path is
# one — can be overwritten by the next run rather than refusing it.
chmod -R u+w "$APPDIR"

# The claim the bundle makes, checked rather than trusted: every library named
# by the app or by anything it carries is either one of the files just copied
# in, or one of the C-runtime names deliberately left out above. Anything else
# is a library this build is borrowing from the machine that made it, and the
# machine that runs it is where that would otherwise be discovered — which for
# the appindicator means a tray icon that never appears, long after a release.
#
# By name rather than by what the loader picks. `LD_LIBRARY_PATH` is what points
# it at the bundle, and it loses to a dependency named as an absolute path —
# which nixpkgs does and a distribution does not, so in the dev shell one or two
# of these are carried and read off the store anyway. That is the sense in which
# a build made here is for the machine that made it, and the file that ships is
# the runner's.
say "Checking that nothing is left outside…"
borrowed=$(
  for file in "$APPDIR/usr/bin/$BINARY" "$APPDIR"/usr/lib/*; do
    ldd "$file" 2> /dev/null | sed -n 's/^[[:space:]]*\(.*\) => not found$/\1/p'
    resolves "$file"
  done | sed 's|.*/||' | sort -u |
    while read -r name; do
      if [[ "$name" =~ $RUNTIME_LIBS ]] || [ -e "$APPDIR/usr/lib/$name" ]; then
        continue
      fi
      printf '%s\n' "$name"
    done
)
[ -z "$borrowed" ] || die "The bundle does not carry what it needs:"$'\n'"$borrowed"

# The packaging assets, at the two places a desktop looks for them: the
# specification's own directories under `usr/share`, for whatever integrates the
# AppImage into a menu, and the AppDir's root, which is where the format itself
# reads the entry and the icon it shows the file under.
#
# **The verb comes off the entry on the way in**, which is this format's own
# wrinkle. `packaging/`'s entry says `Exec=verkstead desktop`, which is right
# for an install that puts the binary on the `PATH` — and wrong here, because
# what integrates an AppImage rewrites `Exec` to name the AppImage and keeps
# what followed it, and the AppImage is already a way into the app: `AppRun`
# below supplies the verb. An entry that said it as well would start
# `verkstead desktop desktop`, which is refused. Nothing outside the entry point
# says the verb, which is the same rule the app's own startup registration
# follows — see `crates/desktop/src/startup/xdg.rs`.
entry() { sed 's|^Exec=verkstead desktop$|Exec=verkstead|' "packaging/$APP_ID.desktop"; }
[ "$(entry | grep -c '^Exec=verkstead$')" -eq 1 ] ||
  die "packaging/$APP_ID.desktop no longer holds the Exec line this takes the verb off."

entry > "$APPDIR/usr/share/applications/$APP_ID.desktop"
cp -r packaging/icons/hicolor "$APPDIR/usr/share/icons/hicolor"
entry > "$APPDIR/$APP_ID.desktop"
cp "packaging/icons/hicolor/256x256/apps/$APP_ID.png" "$APPDIR/$APP_ID.png"
cp "$APPDIR/$APP_ID.png" "$APPDIR/.DirIcon"

# What the runtime executes once it has mounted the image. Every path in it is
# relative to `$APPDIR`, which the runtime sets to wherever it mounted this run
# — the file the human actually has is `$APPIMAGE`, which is what the app reads
# when it registers itself to start with the session, and neither is a path this
# script can know.
#
# `LD_LIBRARY_PATH` rather than an rpath rewritten into every copied library:
# the loader reads it before the RUNPATH a library was built with, so the bundle
# is what satisfies the bundle, and nothing here has to be patched.
#
# Nothing is said about gdk-pixbuf's loadable modules, and deliberately: PNG is
# compiled into that library itself, and the formats those modules add — TIFF,
# XPM, the rest — are ones nothing in this app decodes.
#
# `desktop` sits between the binary and whatever the caller said, because that
# is the half of the binary somebody executing this file is asking for: a
# desktop launcher names a file and has no way to say a verb. The agents' half
# of the same file is reached without passing through here at all — see the
# header.
#
# `APPDIR` is exported rather than only set, which is what the server inside
# reads to find these same libraries again: a session it spawns is handed the
# binary through a launcher of Verkstead's own that points the loader at them,
# because a session gets none of this environment — see
# `crates/server/src/sandbox.rs`. The runtime exports the variable itself, so
# the export is only what covers the other way in, an AppDir run through this
# script directly.
cat > "$APPDIR/AppRun" << 'APPRUN'
#!/bin/sh
set -eu
APPDIR="${APPDIR:-$(dirname "$(readlink -f "$0")")}"
export APPDIR
export LD_LIBRARY_PATH="$APPDIR/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export XDG_DATA_DIRS="$APPDIR/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "$APPDIR/usr/bin/verkstead" desktop "$@"
APPRUN
chmod +x "$APPDIR/AppRun"

# The runtime, kept between runs and checked every time rather than trusted
# because it was already there.
if ! printf '%s  %s\n' "$RUNTIME_SHA256" "$RUNTIME" |
  sha256sum --check --status 2> /dev/null; then
  say "Fetching the AppImage runtime…"
  curl --fail --location --silent --show-error --output "$RUNTIME" "$RUNTIME_URL"
  printf '%s  %s\n' "$RUNTIME_SHA256" "$RUNTIME" | sha256sum --check --status ||
    die "$RUNTIME_URL is not the runtime this script is pinned to."
fi

# The AppDir as a squashfs image, and the runtime in front of it — which is the
# whole of what an AppImage is, and the whole of what `appimagetool` would do
# here. Doing it with `mksquashfs` is what makes this one command in the dev
# shell as much as on a runner: `appimagetool` ships as an AppImage of ordinary
# dynamically-linked binaries, and on a NixOS machine there is no interpreter at
# the path they name.
#
# Owned by root and timestamped at the epoch, so that two builds of the same
# tree produce the same bytes rather than differing by who ran them and when.
# `SOURCE_DATE_EPOCH` is dropped rather than deferred to: the dev shell sets one
# and a runner does not, `mksquashfs` refuses to be told the time twice, and the
# two are meant to produce the same file.
say "Packing $APPIMAGE…"
image="$WORK/verkstead.squashfs"
rm -f "$image"
env -u SOURCE_DATE_EPOCH mksquashfs "$APPDIR" "$image" \
  -root-owned -all-time 0 -mkfs-time 0 -noappend -no-progress -comp zstd
cat "$RUNTIME" "$image" > "$APPIMAGE"
chmod +x "$APPIMAGE"
rm -f "$image"

say ""
say "$APPIMAGE"
