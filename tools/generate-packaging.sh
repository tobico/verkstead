#!/usr/bin/env bash
# Write packaging/ — the desktop entry a launcher lists Verkstead under, and the
# icons it draws beside it — from the one piece of artwork in tools/hammer.
#
# The same rule as tools/generate-icons.sh, which does this for the viewer's
# own: there is one piece of artwork and everything else is output, committed so
# that a build needs nothing but node and cargo, and never hand-edited. Replace
# the artwork and run this.
#
# It writes outside assets/ deliberately. Everything under that directory is
# vite's publicDir — copied whole into the viewer, which is embedded in every
# binary including the headless CLI — and a launcher's icons are neither the
# viewer's to serve nor the CLI's to carry. The artwork it reads is outside it
# for the same reason: it is tools/hammer/render.py's output, and a 1024 square
# nothing serves would be carried inside every binary for nothing. What stays
# under assets/ is the cut icons, which the viewer does serve.
#
# This directory is entirely this script's output: it is rewritten from nothing
# on every run, so a size that stops being generated stops being committed.
# The macOS .icns and the Windows .ico are written here too, from the same
# artwork and the same run.
#
# ImageMagick and desktop-file-utils come from the dev shell, so run this under
# `nix develop` — or as `nix develop --command tools/generate-packaging.sh`.
set -euo pipefail
cd "$(dirname "$0")/.."

# The app id, which is what the tray, the autostart registration and this entry
# are all named for — see `APP_ID` in crates/desktop/src/lib.rs. One string,
# because a desktop told two would have two Verksteads.
APP_ID="net.tobico.Verkstead"

ARTWORK="tools/hammer/verkstead-hammer.png"
OUT="packaging"

rm -rf "$OUT"
mkdir -p "$OUT"

# The desktop entry. It is the launcher's, rather than the autostart entry
# `crates/desktop/src/startup/xdg.rs` writes at runtime: that one names the
# executable it found itself running as and says `--no-open`, because nobody
# wants a browser window at every login. This one is a menu item somebody
# clicked, so it opens the viewer, which is the whole of what clicking Verkstead
# is for.
#
# `Exec` is the bare name and then the verb: the tray app is `verkstead desktop`
# rather than a binary of its own (ADR-0012, amended), and an entry naming the
# path alone would start a Verkstead that printed the Guide. The name rather
# than a path because inside an AppImage the file lives at a path made for that
# one run, and what installs this entry — the desktop's own integration, or a
# package's install step — is what knows where the binary ended up. `Icon` is
# named rather than pointed at for the same reason, and it is the app id, which
# is what the icons below are installed as.
cat > "$OUT/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Verkstead
Comment=The workbench, in the system tray
Exec=verkstead desktop
Icon=$APP_ID
Terminal=false
Categories=Development;
Keywords=agent;coding;sessions;workbench;
EOF

# Read back rather than trusted: an entry a desktop will not parse is one that
# never appears in a menu, and nothing else here would notice.
desktop-file-validate "$OUT/$APP_ID.desktop"

# The sizes a launcher draws at, in the layout the icon theme specification
# names them by — which is the layout they are installed into, so a packaging
# step copies the tree rather than renaming a file per size.
#
# The set stops at 512 because that is the largest size a launcher asks for.
# Everything here is a downscale of a 1024 render, and an icon scaled up is the
# one that looks wrong. 16 through 48 are what a menu and a panel draw, and the
# three above them are what a macOS .icns and a Windows .ico want from the same
# run.
#
# Lanczos, a transparent field and `-strip`, for the reasons
# tools/generate-icons.sh gives: the artwork is fine-lined enough to turn to
# porridge under a softer filter, the field is transparent and stays so, and a
# launcher has no use for the source's colour profile. Stripping is also what
# makes a second run byte-identical to the first — what it takes out is the
# timestamp.
cut() {
  magick "$ARTWORK" -filter Lanczos -background none \
    -resize "${1}x${1}" -strip "$2"
}

for size in 16 24 32 48 64 128 256 512; do
  apps="$OUT/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$apps"
  cut "$size" "$apps/$APP_ID.png"
done

# The macOS icon, which is those same downscales — and one above them — inside
# the one container macOS reads an app's icon out of:
# `Verkstead.app/Contents/Resources`, put there by tools/build-macos-dmg.sh.
#
# Written here rather than handed to `iconutil` because that tool is a Mac's and
# this script runs wherever the dev shell does, while the format is a header and
# one chunk per size: `icns`, the file's own length, and then a four-character
# type, a length and a PNG for each. Nothing about it is Apple-only but the
# reader.
#
# macOS asks for a slot by name rather than for the nearest size, so the @2x
# slots are named as well as the plain ones, each filled by the downscale of the
# pixel size it asks for. That the two are the same file is what an .iconset
# built from this artwork would give it too: there is one piece of artwork and
# everything is a downscale of it.
ICNS_CHUNKS="
icp4 16   16pt
ic11 32   16pt@2x
icp5 32   32pt
ic12 64   32pt@2x
ic07 128  128pt
ic13 256  128pt@2x
ic08 256  256pt
ic14 512  256pt@2x
ic09 512  512pt
ic10 1024 512pt@2x
"

ICNS="$OUT/$APP_ID.icns"

# `ic10` is the one slot the tree above cannot fill. It is 512pt at 2x — 1024
# pixels, the artwork's own size — and no launcher asks for an icon that big, so
# there is no committed downscale to repackage. It is cut here instead, into a
# temporary directory that leaves with the script rather than into `icons/`,
# where a size nothing installs would only be committed and never drawn.
ICNS_1024="$(mktemp -d)/$APP_ID-1024.png"
trap 'rm -rf "$(dirname "$ICNS_1024")"' EXIT
cut 1024 "$ICNS_1024"

# The PNG a chunk carries: for every slot the launcher tree holds, the downscale
# already written above, so the icon a Mac draws and the icon a Linux panel
# draws are the same pixels; and for the one it does not, the 1024 beside it.
icns_png() {
  if [ "$1" -gt 512 ]; then
    printf '%s' "$ICNS_1024"
  else
    printf '%s' "$OUT/icons/hicolor/${1}x${1}/apps/$APP_ID.png"
  fi
}

# Four bytes, most significant first — the only number this format has. Printed
# as escapes for a second printf to write, because that is how a shell puts a
# byte it cannot name into a file.
be32() {
  printf '\\x%02x\\x%02x\\x%02x\\x%02x' \
    $(($1 >> 24 & 255)) $(($1 >> 16 & 255)) $(($1 >> 8 & 255)) $(($1 & 255))
}

# The header's length counts the whole file, so it is known only once every
# chunk's is: eight bytes of header, and eight plus a PNG for each chunk.
total=8
while read -r type size _; do
  [ -n "$type" ] || continue
  total=$((total + 8 + $(wc -c < "$(icns_png "$size")")))
done <<< "$ICNS_CHUNKS"

printf 'icns' > "$ICNS"
printf "$(be32 "$total")" >> "$ICNS"
while read -r type size _; do
  [ -n "$type" ] || continue
  png="$(icns_png "$size")"
  printf '%s' "$type" >> "$ICNS"
  printf "$(be32 $((8 + $(wc -c < "$png"))))" >> "$ICNS"
  cat "$png" >> "$ICNS"
done <<< "$ICNS_CHUNKS"

# Read back, as the desktop entry is: a reader takes the header's length as the
# file's own and stops there when it disagrees, so an icon that is a byte out is
# an icon that never draws and nothing else here would notice.
if [ "$(wc -c < "$ICNS")" -ne "$total" ]; then
  printf '%s\n' "$ICNS is $(wc -c < "$ICNS") bytes and claims $total." >&2
  exit 1
fi

# And the Windows icon, which is those same downscales again inside the one
# container Windows reads an executable's icon out of. It is not copied into a
# bundle the way the .icns is: `crates/desktop/build.rs` compiles it into
# verkstead-desktop.exe — the shim a Start-menu shortcut names — as a resource,
# so Explorer, the taskbar and the window that a dialog opens all draw Verkstead
# out of the file itself.
#
# Written with `magick` rather than by hand, which is where this differs from
# the .icns above: the tool that writes an .icns is a Mac's and the format had
# to be assembled here, while ImageMagick writes an .ico wherever the dev shell
# runs. It is handed the committed downscales rather than the artwork, so the
# icon Windows draws is the same pixels as the icon a Linux panel and a Mac
# draw.
#
# `-type TrueColorAlpha` is the one thing it has to be told: left to itself it
# stores a size whose palette fits in 256 colours as 8-bit, and an 8-bit entry
# in this format carries a one-bit mask rather than an alpha channel — which
# is a hard edge around the artwork at exactly the size the taskbar draws.
#
# The set stops at 256 because the format stops there: an entry says its width
# in one byte, with zero meaning 256, and there is nothing it could say about
# the 512 above it.
ICO="$OUT/$APP_ID.ico"
ICO_SIZES="16 24 32 48 64 128 256"

ico_pngs=()
for size in $ICO_SIZES; do
  ico_pngs+=("$OUT/icons/hicolor/${size}x${size}/apps/$APP_ID.png")
done

magick "${ico_pngs[@]}" -type TrueColorAlpha "$ICO"

# Read back, as everything else here is: what says an .ico is right is the
# number of images in it and the size of each, and a reader that finds one
# entry where seven were meant draws whatever the one is.
written=$(magick identify -format '%w ' "$ICO")
wanted=$(printf '%s ' $ICO_SIZES)
if [ "$written" != "$wanted" ]; then
  printf '%s\n' "$ICO holds the sizes ${written}where it should hold ${wanted}" >&2
  exit 1
fi
