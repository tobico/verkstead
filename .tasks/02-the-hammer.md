# 02. The hammer

## What to build

The hammer, modelled in Blender over the MCP that task 01 wired, to the look
the human settled, reviewed by them a render at a time, and saved as the blend
file that is from now on the mark's source of truth. This is the one task that
cannot share a session with the wiring: the tools are read at session start, so
this session is the first that has them.

**How the session works.** It starts `tools/hammer/serve.py` in the background
as task 01 left it, and models through the MCP tools — `get_scene_info`,
`execute_blender_code` and the rest — rather than through a script that rebuilds
the scene from nothing. What it checks its work against is a render, not a
viewport screenshot: a headless Blender has no viewport, and Cycles on this
CPU renders a 512px preview in a tenth of a second and a denoised 1024px final in
under two. The scene is saved to `tools/hammer/verkstead-hammer.blend` as it
goes, and the camera, the lights and the film settings live in that file, so the
render script has nothing to decide but size, samples and where to write.

**The look, as settled.** Same kind of hammer as the icon today and the same
palette, with the pose and proportions improved rather than copied:

- A **lump hammer** — a big rectangular head, about a third of the diagonal, so
  it still reads at 32 pixels; not an engineer's or a claw hammer.
- **Three-quarter view from above**, head at the bottom left, handle running to
  the top right, and the handle steepened so the hammer fills the square corner
  to corner. The drawing today fills the width and only 82% of the height; the
  favicon can use that room. The art still runs to the square's edges, so the
  manifest's `any` stays true and nothing claims `maskable`.
- **The head**: generous chamfers on every edge, which are what catch the light;
  striking faces slightly domed so each carries a highlight; the wedge visible
  in the eye, in dark iron.
- **Materials, all procedural** — no downloaded textures, so the blend file needs
  nothing but Blender. The head is satin blue-grey steel with faint brushing, in
  the drawing's palette: a dark `#1C253B`, a mid `#545C6D`, a highlight around
  `#B8BBC5`. The handle is a procedural wood grain in the drawing's orange-brown,
  `#BE612C` through `#CB8E6D`, under a light varnish. The wedge is the head's
  iron, darker.
- **Lighting**: soft studio — a key from the upper left where the drawing's
  highlight sits, a fill and a rim — on a transparent film with no ground
  shadow, so it composites on the light sidebar and the dark iOS tile alike.

**The render script.** `tools/hammer/render.py`, run as `blender -b
tools/hammer/verkstead-hammer.blend --python tools/hammer/render.py`, renders the
saved file at 1024 square with Cycles on the CPU, denoised, transparent, 8-bit
RGBA, to `assets/icons/verkstead-hammer.png`, and stops there: the two cut
scripts stay the next step, as the docs say. It is written in this task rather
than the next because what the human reviews should be exactly what the
pipeline will produce.

**The review loop.** Each round renders a candidate over the artwork's own path,
uncommitted, and asks a Question Set that names the file — the Set's Diff omits
a binary's contents, so the human looks at the Worktree, and they said they
would. The candidate sits where the icon will live, so what they see is the real
thing at the real place. A session cannot end with uncommitted changes, so when
the human says a render is the one, the artwork is restored to the committed
drawing before the task ends; task 03 is what replaces it. The blend file and
the render script are what this task commits.

Rejected: a Python script as the model's source (the human chose the blend file
plus a render script); reproducing the drawing's painted grain streaks
(procedural grain instead); a contact shadow (none, as now); a viewport
screenshot loop (no viewport headless, and renders are as fast).

## Acceptance criteria

- [ ] `tools/hammer/verkstead-hammer.blend` is committed, `*.blend` is marked
      binary in `.gitattributes`, and the file opens in Blender 5.1 with the
      camera, three lights, the head, handle and wedge in it and nothing linked
      from outside.
- [ ] `blender -b tools/hammer/verkstead-hammer.blend --python
      tools/hammer/render.py` writes a 1024×1024 8-bit RGBA PNG to the artwork's
      path, transparent outside the hammer, with the hammer touching the
      square's edges.
- [ ] The human has said, on a Question Set naming the candidate, that the
      render is the one; the task ends with the artwork restored to the
      committed drawing and no uncommitted files.
