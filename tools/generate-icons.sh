#!/usr/bin/env bash
# Downscale the artwork in assets/icons into the sizes the favicon, the manifest
# and iOS ask for.
#
# There are three pieces of artwork, because the one square that serves a
# launcher does not serve a 32px tab or an iOS home screen. Every other icon in
# that directory is this script's output. They are committed so that a build
# needs nothing but node and cargo, but they are never hand-edited: replace the
# artwork and run this. The script lives outside assets/ because everything in
# there is copied into the served site root, and a build script is not part of
# the site.
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
  magick "$1" -filter Lanczos -background none -resize "$2x$2" -strip "$3"
}

# The full mark, on a transparent field: the manifest's icons, and the sidebar's
# at 3rem. Drawn far larger than anything serving it.
shrink verkstead.png 192 icon-192.png
shrink verkstead.png 512 icon-512.png

# The favicon, at the size a tab actually draws — and cut from the hammer alone,
# because at 32px the full mark is a grey smudge with confetti on it. A browser
# will take any PNG and scale it, but nothing optimising for speed is going to
# rescue artwork that has too much in it for the size.
shrink verkstead-hammer.png 32 icon-32.png

# iOS ignores the manifest's icons and takes this one, at this exact size — and
# it ignores transparency too, compositing whatever it is given onto black. So
# it gets the variant that was drawn with a field of its own, and `-alpha off`
# so nothing downstream can hand iOS a channel to composite.
magick verkstead-bg.jpg -filter Lanczos -resize 180x180 \
  -alpha off -strip apple-touch-icon.png
