#!/usr/bin/env bash
# Build Verkstead-universal.dmg: the one `verkstead` binary with the tray in it,
# as a macOS application bundle built for both Macs at once, inside the disk
# image a download arrives as.
#
# **What is inside is the whole of Verkstead** — built with the `desktop`
# feature, which is that binary's default (ADR-0012, as amended). So the file a
# human double-clicks and the file a session it spawned runs `ask` with are one
# file, which is the invariant the sandbox stands on. Each half is reached its
# own way: the bundle's executable is a launcher script that supplies the
# `desktop` verb, because a bundle names an executable and has nowhere to write
# a command line for it, and a session runs the binary beside that script by
# path, saying a verb of its own.
#
# The Linux artifact is an AppImage because the tray is drawn over system
# libraries that have to be carried — see tools/build-appimage.sh, whose
# `AppRun` supplies the same verb for the same reason. None of that applies
# here: AppKit is the operating system, so the bundle holds the binary, the
# launcher and the icon and nothing else, and what makes it an artifact is the
# .app layout around it and the dmg around that.
#
# Run it on a Mac, in the dev shell or on a runner. It takes everything from the
# working tree — the viewer from `web/dist`, the icon from `packaging/` — and
# leaves one file at a fixed path, printed at the end, which is what the release
# workflow uploads.
#
# Universal, per the stage's decision: one download runs on both Macs. Both
# halves are built here rather than fetched, because an Apple target
# cross-compiles from an Apple host — the SDK carries both architectures and
# clang is told which by `-arch` — so whichever runner this leg is given can
# build the other half and `lipo` the two together.
set -euo pipefail
cd "$(dirname "$0")/.."

APP_ID="net.tobico.Verkstead"
# Three names where there was one: the package cargo is asked for, the file it
# leaves — the tray app being a verb of the CLI's binary rather than a binary of
# its own — and the script beside it that `CFBundleExecutable` names.
PACKAGE="verkstead-cli"
BINARY="verkstead"
# Not `Verkstead`, which is the name a bundle's executable would otherwise want:
# HFS+ and the APFS a Mac formats itself with are both case-insensitive, so a
# script called that could not sit in one directory with a binary called
# `verkstead` at all — one of the two would be the other.
LAUNCHER="Verkstead-launcher"

# The two Apple targets the release already builds the bare CLI for, joined into
# one file below — each paired with the name `lipo` knows that architecture by,
# which is rustc's for one of them and not for the other.
TARGETS="aarch64-apple-darwin:arm64 x86_64-apple-darwin:x86_64"

# Where everything this script makes goes, under the directory cargo already
# writes to, so that one `.gitignore` line covers it and `cargo clean` takes it
# away.
WORK="target/macos"
APP="$WORK/Verkstead.app"
STAGE="$WORK/dmg"
DMG="$WORK/Verkstead-universal.dmg"

# A name said before an ellipsis is braced — `${target}…` rather than
# `$target…`. The bash a Mac runs reads the first byte of that character as part
# of the name, so an unbraced one is a variable nobody set and `set -u` ends the
# script on the line that was only reporting progress.
say() { printf '%s\n' "$*"; }
die() {
  printf '%s\n' "$*" >&2
  exit 1
}

[ "$(uname -s)" = "Darwin" ] ||
  die "This builds a macOS application bundle and has to run on a Mac."

# The viewer, which is embedded at compile time rather than read at runtime —
# see `crates/server/src/viewer.rs`. The embed is `allow_missing`, so a tree
# without it builds an app that serves a 503 and says nothing about why, which
# is exactly the artifact nobody wants to find out about after a release.
[ -f web/dist/index.html ] ||
  die "web/dist is empty: build the viewer first, with (cd web && pnpm build)."

# The icon, which is generated and committed rather than built here — see
# tools/generate-packaging.sh, which writes it from the same artwork the Linux
# launcher icons come from.
ICNS="packaging/$APP_ID.icns"
[ -f "$ICNS" ] ||
  die "$ICNS is not there: run tools/generate-packaging.sh."

for tool in cargo rustc lipo codesign plutil hdiutil ditto; do
  command -v "$tool" > /dev/null ||
    die "$tool is needed to build the dmg and is not on the PATH."
done

# Both halves have to be compilable before either is compiled: a run that built
# one target and then discovered the other was never installed would have spent
# the long half of the build to say so.
for entry in $TARGETS; do
  target="${entry%%:*}"
  libdir=$(rustc --print target-libdir --target "$target" 2> /dev/null || true)
  [ -d "$libdir" ] ||
    die "$target has no standard library here: rustup target add $target"
done

# The viewer is compiled in through `include_bytes!`, one file at a time, and
# cargo rebuilds when one of them changes — but a `web/dist` that was empty at
# the last build left nothing to notice, so a binary built before the viewer was
# would be reused with no viewer in it, and this script would wrap it. Touching
# the file that declares the embed is what says otherwise.
touch crates/server/src/viewer.rs

# The package by name rather than the whole workspace, for the reason
# `nix/verkstead-source.nix` gives: `verkstead-render`'s own default features
# turn on the TypeScript emitter, which is a test's business. And nothing here
# turns the `desktop` feature off, unlike every headless build of the same
# package — this is the artifact that wants the tray half, and it wants the
# other half in the same file.
halves=()
for entry in $TARGETS; do
  target="${entry%%:*}"
  say "Building $BINARY for ${target}…"
  cargo build --release --locked --package "$PACKAGE" --target "$target"
  halves+=("target/$target/release/$BINARY")
done

rm -rf "$APP" "$STAGE"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# One binary out of the two, which is what makes the download universal: `lipo`
# copies each half whole and writes a header in front of them saying which is
# which, and the loader takes the half this Mac runs.
say "Joining the two halves…"
lipo -create -output "$APP/Contents/MacOS/$BINARY" "${halves[@]}"

# Executable, said rather than left to what `lipo` copied: a bundle whose
# executable is not one is a bundle macOS refuses to launch.
chmod +x "$APP/Contents/MacOS/$BINARY"

# The claim that makes, checked rather than trusted: a `lipo` given one input
# writes a perfectly good single-architecture file, and the Mac that could not
# run it is the one nobody testing this has.
for entry in $TARGETS; do
  arch="${entry##*:}"
  lipo -archs "$APP/Contents/MacOS/$BINARY" | tr ' ' '\n' | grep -qx "$arch" ||
    die "The binary has no $arch in it."
done

# The launcher, which is what `CFBundleExecutable` names below and so what
# Launch Services starts. A bundle names an executable and has nowhere to write
# a command line for it — a double-click passes none, and neither does anything
# else that opens an app — so the verb is supplied here, which is the same job
# `tools/build-appimage.sh`'s `AppRun` does. And the whole of the job: the other
# half of this binary is reached by running it directly, without passing through
# this script at all.
#
# `dirname "$0"` rather than a path written in: Launch Services starts this by
# its full path, so that is the directory the binary is in wherever the app was
# dragged to. Nothing is stripped from `"$@"` — the `-psn_` argument macOS used
# to hand a bundle went in 10.9, long before the 11.0 this bundle's floor is.
say "Writing the launcher…"
cat > "$APP/Contents/MacOS/$LAUNCHER" << LAUNCHER_EOF
#!/bin/sh
set -eu
exec "\$(dirname "\$0")/$BINARY" desktop "\$@"
LAUNCHER_EOF
chmod +x "$APP/Contents/MacOS/$LAUNCHER"

cp "$ICNS" "$APP/Contents/Resources/$APP_ID.icns"

# The version the plist claims, asked of cargo rather than read off `Cargo.toml`
# — `cargo pkgid` answers for the package that was just built, so the number in
# Finder's Get Info is the number the binary was built at.
VERSION="$(cargo pkgid --package "$PACKAGE")"
VERSION="${VERSION##*@}"

# What Launch Services reads before the process starts. `LSUIElement` is the one
# key this app turns on: Verkstead is a menu-bar app with no window behind the
# icon, and without it macOS gives the bundle a Dock tile and an application
# menu it has nothing to put in. The running app asks AppKit for the same thing
# — see `toolkit::start` and its Accessory activation policy — because a binary
# run out of a checkout has no bundle to read this from; here it is settled
# before the app is launched rather than a tile that appears and goes.
#
# `LSMinimumSystemVersion` is the Apple-silicon half's floor, which rustc builds
# that target to and which is the higher of the two. One app gets one number,
# and the honest one is the one both halves meet.
say "Writing the bundle…"
cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>$LAUNCHER</string>
	<key>CFBundleIconFile</key>
	<string>$APP_ID</string>
	<key>CFBundleIdentifier</key>
	<string>$APP_ID</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>Verkstead</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>$VERSION</string>
	<key>CFBundleVersion</key>
	<string>$VERSION</string>
	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>LSUIElement</key>
	<true/>
</dict>
</plist>
EOF

# Read back rather than trusted, as the desktop entry is: a plist macOS will not
# parse is a bundle it refuses to launch, and the shell that wrote it has no
# opinion about what it wrote.
plutil -lint "$APP/Contents/Info.plist" > /dev/null ||
  die "The Info.plist this wrote is not a property list."

# Ad-hoc signed, which is not the signing this stage decided against. A
# Developer ID is what gets an app past Gatekeeper, and there is none — what a
# downloader has to do about that is the install story's to document. This is
# the other thing a signature is for: on Apple silicon the kernel refuses to
# execute a binary carrying no signature at all, so an unsigned bundle would not
# start on half the Macs it is built for.
#
# **The binary is signed first, and on its own**, which is what the launcher
# costs: `codesign` over a bundle signs the executable the plist names and
# seals everything else as a resource, and the executable the plist names is
# now a shell script. So the Mach-O half — the one the kernel has the opinion
# about — is signed before the bundle is, and the bundle then seals the signed
# file rather than an unsigned one. Signing the bundle around it is what takes
# the Info.plist and the icon into what is sealed, as it always was.
say "Sealing the bundle…"
codesign --force --sign - "$APP/Contents/MacOS/$BINARY"
codesign --force --sign - "$APP"
codesign --verify --strict "$APP" ||
  die "The bundle did not verify against the signature just written."
codesign --verify --strict "$APP/Contents/MacOS/$BINARY" ||
  die "The binary inside did not verify against its own signature."

# What the mounted image shows: the app, and the folder to drag it into. The
# symlink is the whole of the installer — a dmg is a disk, and Finder's copy is
# what installing a Mac app has always been.
#
# `ditto` rather than `cp -R`, which is the launcher's second cost: a signature
# over a script has nowhere inside the file to live, so `codesign` writes it to
# an extended attribute — and `cp` carries those only when it is asked to,
# where `ditto` is the copy that carries everything. A signature lost here
# would not be an app that starts unsigned; it would be an app whose seal
# disagrees with what is under it, which is the "damaged" a Mac refuses to open
# rather than the "unidentified developer" it offers a way past.
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/Verkstead.app"
ln -s /Applications "$STAGE/Applications"

# The image itself: compressed, read-only, and HFS+ because that is the
# filesystem every macOS this app claims can mount. Two builds of the same tree
# do not produce the same bytes — `hdiutil` stamps the volume with the time it
# was made and offers nothing to say otherwise — so the dmg is checked by what
# comes out of it rather than by its hash.
say "Packing ${DMG}…"
rm -f "$DMG"
hdiutil create -volname Verkstead -srcfolder "$STAGE" \
  -fs HFS+ -format UDZO -ov -quiet "$DMG"

say ""
say "$DMG"
