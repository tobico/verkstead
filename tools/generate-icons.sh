#!/usr/bin/env bash
# Downscale the artwork in assets/icons into the sizes the favicon, the manifest
# and iOS ask for.
#
# There is one piece of artwork, and every other icon in that directory is this
# script's output. They are committed so that a build needs nothing but node and
# cargo, but they are never hand-edited: replace the artwork and run this. The
# script lives outside assets/ because everything in there is copied into the
# served site root, and a build script is not part of the site.
#
# ImageMagick comes from the dev shell, so run this under `nix develop` — or as
# `nix develop --command tools/generate-icons.sh`.
set -euo pipefail
cd "$(dirname "$0")/../assets/icons"

# The chrome's colour, from the web manifest's `theme_color` and the document's
# `theme-color` tag. Only iOS needs it — see below — but it is the same colour
# so that the tile reads as the app's own rather than as a third one.
FIELD="#21201e"

# Lanczos rather than the default, because the artwork is fine-lined enough that
# the tracks on it turn to porridge under a softer filter; `-background none` so
# the field stays transparent through the resize; and `-strip` because nothing
# serving a 32px icon has any use for the source's colour profile.
shrink() {
  magick "$1" -filter Lanczos -background none -resize "$2x$2" -strip "$3"
}

# The mark, on a transparent field: the favicon at the size a tab actually
# draws, and the manifest's icons, which are also what the sidebar draws at
# 3rem. All far smaller than the source.
shrink verkstead-hammer.png 32 icon-32.png
shrink verkstead-hammer.png 192 icon-192.png
shrink verkstead-hammer.png 512 icon-512.png

# iOS ignores the manifest's icons and takes this one, at this exact size. It
# ignores transparency too, compositing whatever it is given onto black, so this
# is the one output with a field under it. It is also the one output with a
# margin: iOS rounds the tile's corners itself, and the head and the handle both
# run to the edge of the artwork's square, so drawing it at full bleed would
# hand the mask something to cut off.
magick verkstead-hammer.png -filter Lanczos -background "$FIELD" \
  -resize 160x160 -gravity center -extent 180x180 \
  -alpha off -strip apple-touch-icon.png
