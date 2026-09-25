# The hammer in Blender

The mark is a hammer drawn by a generative model at 545 pixels, and every icon
the app draws — favicon, manifest, iOS tile, tray, launcher tree, icns and ico —
is a downscale of it. This replaces the drawing with a render: the same lump
hammer, modelled in Blender over the Blender MCP inside a Verkstead session,
saved as a blend file the repository keeps, and rendered at 1024 pixels by a
script that anybody with a Blender can run again. The two cut scripts then do
what they always did over the new artwork, and the icns gains the 1024 slot the
old source was too small to fill.

The grilling settled where Blender runs and what is kept. It runs headless in
the session's Sandbox, on the CPU — no display and no GPU reach it, and a final
render of a stand-in scene took under two seconds, so nothing waits on
hardware. The blend file is the source of truth and the render script is its
one consumer; nothing about the model lives in a Python script that rebuilds it
from nothing. The MCP wiring is committed to the repository for good, because a
session reads its project MCP file when it starts and Verkstead strips the
account's own servers, so a committed file is the only way a later session gets
the tools back. The look — pose, proportions, chamfers, wedge, materials and
light — is written into task 02.

## Tasks

- [x] 01: The Blender MCP wiring — [details](01-the-blender-mcp-wiring.md)
- [x] 02: The hammer — [details](02-the-hammer.md)
- [x] 03: The icon, replaced — [details](03-the-icon-replaced.md)
