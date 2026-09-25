# 03. The icon, replaced

## What to build

The render becomes every icon the app draws. This is the slice that changes
what a human sees: the favicon in the tab, the sidebar's mark, the installed
app's tile, the tray, the launcher's menu entry, the Mac's dock and the Windows
taskbar all draw the render instead of the drawing, cut by the same two scripts
as before.

**The artwork.** `tools/hammer/render.py` from task 02 writes
`assets/icons/verkstead-hammer.png` at 1024 square. That file is committed, as
the drawing was, and is the one piece of artwork; the blend file is what it is
made from, and the docs say so.

**The cuts.** `tools/generate-icons.sh` and `tools/generate-packaging.sh` re-run
over it. Both are downscales from a source that is now 1024 rather than 545, so
what each says about its source moves with it: the packaging script stops at
512 "because the artwork is 545", and leaves the icns's `ic10` slot — 512pt at
2x, 1024 pixels — empty for the same reason. That slot is filled now, from a
1024 downscale that is the artwork itself, and the comments that explained the
gap go. The sizes a launcher asks for do not change. The ico still stops at 256,
because the format does.

**What has to stay true.** The tray reads `icon-192.png` and refuses anything
that is not 8-bit RGBA, which a Lanczos downscale of an RGBA render is. The
viewer's test walks the manifest's icons and asserts none claims `maskable`,
which holds because the hammer still runs to the square's edges. The iOS tile
keeps its field and its margin, because iOS composites onto black and rounds the
corners itself. Stripping keeps a second run of either script byte-identical to
the first, which is what makes the committed outputs reviewable.

**The docs.** The artwork table in `docs/development.md` describes three pieces
of artwork and two files that were deleted when every icon was cut from the
hammer alone; the commit that did that did not update it. It becomes one row —
the render, what it is cut into, and that it is made from the blend file by the
render script, with Blender on the machine rather than in the dev shell — and
the dev loop's list of tools gains the render command beside the two cut
scripts. The paragraph about the iOS tile's colour, which describes a field the
artwork no longer carries, says what the script does today.

Rejected: keeping the render at 545 to leave the scripts alone (the human chose
1024 and the icns slot); rendering the cuts in Blender at each size (the cut
scripts exist, and one downscale filter for every platform is the point of
them).

## Acceptance criteria

- [ ] `assets/icons/verkstead-hammer.png` is the 1024-square render, and every
      file under `assets/icons/` and `packaging/` is its output from the two
      scripts, run from the dev shell, with a second run changing nothing.
- [ ] The icns carries `ic10` at 1024, its header length agrees with the file,
      and the packaging script's comments no longer describe a 545 source.
- [ ] `cargo test` and the web suite are green — the tray's RGBA check, the
      manifest's icons and the favicon links included — and the sidebar, the
      favicon and the tray draw the render.
- [ ] `docs/development.md` describes one piece of artwork, how it is rendered
      and how it is cut, and nothing it says about the icons is untrue.
