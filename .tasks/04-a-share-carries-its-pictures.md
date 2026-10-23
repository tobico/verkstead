# 04. A share carries its pictures

## What to build

A share is one file read with no server behind it, so a picture fetched by id
would be a figure with nothing in it. As the server writes a share, it fills
every picture in — each question set's Diff blocks and each commit's pane —
reading the bytes out of the repository by the rules task 01 serves them by:
the repository the block or the commit names, the worktree standing in for an
after blob not yet staged, the 2 MB cap. Each is written into its placeholder
as a `data:` URI, so the viewer in the share draws it without fetching, and the
modal of task 03 opens it as it does in the workbench. A picture that cannot be
read at share time is left as the viewer finds an unfetchable one — the SVG's
hunks, a line for a raster — and the share says nothing more about it.

Build it so the renderer is handed a reader of blobs rather than having its
HTML rewritten afterwards: the workbench's render hands it none and gets
placeholders, the share's hands it the repository and gets pictures inlined.
One rendering, two readers.

Pictures are the one thing that can make a share large, and a share has no cap
of its own: the per-file cap bounds it, as it bounds the workbench.

## Acceptance criteria

- [ ] A share taken of a conversation whose sets and commits hold pictures draws every one of them with the server stopped, Before and After pairs included, and opens them at full size.
- [ ] A picture whose blob is gone at share time draws in the share what the workbench draws for an unfetchable one.
- [ ] No share fetches anything: the share build's suite proves the file holds its pictures and makes no request for them.
- [ ] The server's sharing tests cover the inlining and the fallback.
