# 01. One buffer behind every view

## What to build

**The Monaco buffer moves above the pane**, to sit with the readings and the
tabs rather than inside the editor that draws it. Today an editor makes its own
buffer when its tab is drawn and disposes it when the tab goes, and the text
the human typed is kept beside it as a plain string. That cannot hold two
views: a buffer is registered at the file's own path, and Monaco refuses a
second one at that address — so the same file in two groups is impossible until
this moves.

So the keeping that already holds the tabs, the readings and the unsaved text
holds the buffers too, one per open file path, made when a file is first opened
and disposed when its **last** view closes. An editor is handed one rather than
making one, and everything that reads or writes the text — the dot on the tab,
Ctrl+S, **Reload** and **Keep mine** — goes through the buffer instead of
through a string beside it. The dirty comparison stays what it is: the buffer's
text against what the disk said at the version it said it at.

**This is a move rather than a change of behaviour**, with one visible gain
that is the proof it worked: an editor is no longer made afresh every time a
tab is turned away from and back to, so the caret and the undo stack survive
the turn. Nothing else the human can see should differ.

**The suite must still not mount the real editor.** The stub standing in for
Monaco is the seam `Code.tsx` and `Editor.tsx` import; whatever shape the
buffers take has to stay askable of that stub, and the run has to stay well
clear of the heap limit.

## Acceptance criteria

- [ ] A tab turned away from and back to keeps its caret and its undo stack.
- [ ] The dot, Ctrl+S, the refused-stale-save bar and both its presses behave
      exactly as they did before.
- [ ] A file's buffer is disposed when its last view closes, and opening the
      file again is a fresh read and a fresh buffer.
- [ ] The vitest suite passes without the real Monaco package in the room.
