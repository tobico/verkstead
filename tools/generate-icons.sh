#!/usr/bin/env bash
# Downscale assets/icons/verkstead.png into the sizes the favicon, the manifest
# and iOS ask for.
#
# The artwork is one square PNG with a transparent field, drawn far larger than
# anything serves it, and every other icon in that directory is this script's
# output. They are committed so that a build needs nothing but node and cargo,
# but they are never hand-edited: replace the artwork and run this. The script
# lives outside assets/ because everything in there is copied into the served
# site root, and a build script is not part of the site.
#
# ImageMagick comes from the dev shell, so run this under `nix develop` — or as
# `nix develop --command tools/generate-icons.sh`.
set -euo pipefail
cd "$(dirname "$0")/../assets/icons"

# Lanczos rather than the default, because the artwork is fine-lined enough that
# the tracks on it turn to porridge under a softer filter; `-background none` so
# the field stays transparent through the resize; and `-strip` because nothing
# serving a 32px icon has any use for the source's colour profile.
shrink() {
  magick verkstead.png -filter Lanczos -background none -resize "$1x$1" -strip "$2"
}

# The favicon, at the size a tab actually draws. A browser will take any PNG and
# scale it, but this artwork has too much in it to survive being scaled by
# something optimising for speed.
shrink 32 icon-32.png
shrink 192 icon-192.png
shrink 512 icon-512.png

# iOS ignores the manifest's icons and takes this one, at this exact size — and
# it ignores transparency too, compositing whatever it is given onto black. So
# this is the one icon with a field of its own, and the field is the colour the
# browser chrome already is, read out of the manifest rather than written down
# twice.
field=$(sed -n 's/.*"theme_color": *"\([^"]*\)".*/\1/p' ../manifest.webmanifest)
[ -n "$field" ] || {
  echo "no theme_color in ../manifest.webmanifest" >&2
  exit 1
}
magick verkstead.png -filter Lanczos -background none -resize 180x180 \
  -background "$field" -flatten -alpha off -strip apple-touch-icon.png
