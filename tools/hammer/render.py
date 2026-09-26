# Render the hammer to the artwork the two icon scripts cut from:
#
#   $ blender -b tools/hammer/verkstead-hammer.blend --python tools/hammer/render.py
#
# The blend file beside this one is the mark's source of truth. The camera, the
# five lights, the film and every material live in it, so this script decides
# only how big, how many samples and where to write — which is why a render from
# here and a render from Blender's own window are the same picture. Nothing here
# rebuilds the model, and nothing here should start to: change the hammer in
# Blender and save the file.
#
# Cycles on the CPU, because there is no GPU in a Verkstead Sandbox and none is
# wanted: a denoised 1024px square takes a couple of seconds. The output is
# 8-bit RGBA over a transparent film, because every icon in the repository is cut
# from it and the manifest's `any` needs the field to stay clear.
#
# Then it stops. tools/generate-icons.sh and tools/generate-packaging.sh are the
# next two steps, and they are run by hand as the development docs say.
#
#   $ blender -b tools/hammer/verkstead-hammer.blend --python tools/hammer/render.py -- --size 2048
#
import argparse
import os
import sys

import bpy

SIZE = 1024
SAMPLES = 256
ARTWORK = "verkstead-hammer.png"


def script_args():
    """Blender's own arguments stop at `--`; ours are what follows it."""
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="render.py")
    parser.add_argument("--size", type=int, default=SIZE,
                        help="the square's side in pixels (default %d)" % SIZE)
    parser.add_argument("--samples", type=int, default=SAMPLES,
                        help="Cycles samples before denoising (default %d)" % SAMPLES)
    parser.add_argument("--out", default=None,
                        help="where to write, instead of " + ARTWORK)
    return parser.parse_args(argv)


def artwork_path():
    """The artwork, as an absolute path: a relative one is read against Blender's
    own working directory, and `//` needs the blend file that is opening us.

    It lands beside the blend it came out of rather than in `assets/icons`,
    because everything under `assets/` is vite's `publicDir` — copied into the
    viewer and carried inside every binary — and a 1024 square nothing serves
    has no business there. The two cut scripts read it from here."""
    return os.path.join(os.path.dirname(bpy.data.filepath), ARTWORK)


def main():
    args = script_args()
    scene = bpy.context.scene

    scene.render.resolution_x = scene.render.resolution_y = args.size
    scene.render.resolution_percentage = 100
    scene.cycles.samples = args.samples

    scene.render.image_settings.file_format = 'PNG'
    scene.render.image_settings.color_mode = 'RGBA'
    scene.render.image_settings.color_depth = '8'
    scene.render.filepath = os.path.abspath(args.out) if args.out else artwork_path()

    bpy.ops.render.render(write_still=True)
    print("render.py: %dx%d, %d samples, written to %s"
          % (args.size, args.size, args.samples, scene.render.filepath))
    return 0


if __name__ == "__main__":
    sys.exit(main())
