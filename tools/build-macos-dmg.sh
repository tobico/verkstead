#!/usr/bin/env bash
# Build Verkstead-universal.dmg: the desktop app as a macOS application bundle,
# built for both Macs at once, inside the disk image a download arrives as.
#
# The Linux artifact is an AppImage because the tray is drawn over system
# libraries that have to be carried — see tools/build-appimage.sh. None of that
# applies here: AppKit is the operating system, so the bundle holds the binary
# and its icon and nothing else, and what makes it an artifact is the .app
# layout around it and the dmg around that.
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
BINARY="verkstead-desktop"

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

for tool in cargo rustc lipo codesign plutil hdiutil; do
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

halves=()
for entry in $TARGETS; do
  target="${entry%%:*}"
  say "Building $BINARY for $target…"
  cargo build --release --locked --package "$BINARY" --target "$target"
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

cp "$ICNS" "$APP/Contents/Resources/$APP_ID.icns"

# The version the plist claims, asked of cargo rather than read off `Cargo.toml`
# — `cargo pkgid` answers for the package that was just built, so the number in
# Finder's Get Info is the number the binary was built at.
VERSION="$(cargo pkgid --package "$BINARY")"
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
	<string>$BINARY</string>
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
# start on half the Macs it is built for. Signing the bundle rather than the
# binary is what takes the Info.plist and the icon into what is sealed.
say "Sealing the bundle…"
codesign --force --sign - "$APP"
codesign --verify --strict "$APP" ||
  die "The bundle did not verify against the signature just written."

# What the mounted image shows: the app, and the folder to drag it into. The
# symlink is the whole of the installer — a dmg is a disk, and Finder's copy is
# what installing a Mac app has always been.
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/Verkstead.app"
ln -s /Applications "$STAGE/Applications"

# The image itself: compressed, read-only, and HFS+ because that is the
# filesystem every macOS this app claims can mount. Two builds of the same tree
# do not produce the same bytes — `hdiutil` stamps the volume with the time it
# was made and offers nothing to say otherwise — so the dmg is checked by what
# comes out of it rather than by its hash.
say "Packing $DMG…"
rm -f "$DMG"
hdiutil create -volname Verkstead -srcfolder "$STAGE" \
  -fs HFS+ -format UDZO -ov -quiet "$DMG"

say ""
say "$DMG"
