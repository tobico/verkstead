#!/usr/bin/env bash
# Write packaging/ — the desktop entry a launcher lists Verkstead under, and the
# icons it draws beside it — from the one piece of artwork in assets/icons.
#
# The same rule as tools/generate-icons.sh, which does this for the viewer's
# own: there is one piece of artwork and everything else is output, committed so
# that a build needs nothing but node and cargo, and never hand-edited. Replace
# the artwork and run this.
#
# It writes outside assets/ deliberately. Everything under that directory is
# vite's publicDir — copied whole into the viewer, which is embedded in every
# binary including the headless CLI — and a launcher's icons are neither the
# viewer's to serve nor the CLI's to carry. The one thing that stays there is
# the artwork itself, which is the viewer's source too.
#
# This directory is entirely this script's output: it is rewritten from nothing
# on every run, so a size that stops being generated stops being committed.
# Stages 04 and 05 add the macOS .icns and the Windows .ico here, from the same
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

ARTWORK="assets/icons/verkstead-hammer.png"
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
# `Exec` is the bare name rather than a path: inside an AppImage the file lives
# at a path made for that one run, and what installs this entry — the desktop's
# own integration, or a package's install step — is what knows where the binary
# ended up. `Icon` is named rather than pointed at for the same reason, and it
# is the app id, which is what the icons below are installed as.
cat > "$OUT/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Verkstead
Comment=The workbench, in the system tray
Exec=verkstead-desktop
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
# The set stops at 512 because the artwork is 545 square: everything here is a
# downscale, and an icon scaled up is the one that looks wrong. 16 through 48
# are what a menu and a panel ask for, and the three above them are what a
# macOS .icns and a Windows .ico want from the same run.
#
# Lanczos, a transparent field and `-strip`, for the reasons
# tools/generate-icons.sh gives: the artwork is fine-lined enough to turn to
# porridge under a softer filter, the field is transparent and stays so, and a
# launcher has no use for the source's colour profile. Stripping is also what
# makes a second run byte-identical to the first — what it takes out is the
# timestamp.
for size in 16 24 32 48 64 128 256 512; do
  apps="$OUT/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$apps"
  magick "$ARTWORK" -filter Lanczos -background none \
    -resize "${size}x${size}" -strip "$apps/$APP_ID.png"
done
