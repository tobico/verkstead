# 04. Editors follow

## What to build

On a `files` Nudge the pane re-reads the version of every file it has open. The
version is what tells a real change from a touch: where it matches, nothing
happens at all — a build that rewrote a file with the bytes it already had, a
timestamp moved, another root touched entirely.

Where it differs, what happens is the buffer's own.

- **A clean buffer takes the disk's text silently**, keeping its caret as near
  as it can and drawing no bar. Every view of the file shows it, there being one
  buffer under them however many groups it is open in.
- **A dirty one keeps the human's text and raises the bar at once** — the
  Reload / Keep mine bar stage 01 built for a refused save, drawn now the moment
  the disk moves rather than at the next Ctrl+S. Both presses are the reads they
  already are: Reload takes the disk's text and its version, Keep mine takes the
  version alone.
- **A file that has gone keeps its tab**, read-only with its text still in it,
  the way a file deleted from the tree does.

**The page's own save is not the disk moving under it.** A write that landed
answers with the version it made, and the Nudge that write produced must not put
a bar up over the human's own typing.

## Acceptance criteria

- [ ] An agent's edit to a clean open file appears without a keypress, with the
      caret near where it was and no bar drawn.
- [ ] The same edit to a dirty file raises the bar at once with the human's text
      intact; Reload takes the disk's text, Keep mine leaves theirs.
- [ ] A file rewritten with the bytes it already had changes nothing about its
      tab, and neither does a Nudge about a file nothing has open.
- [ ] A save made in the pane does not raise a bar on the file it just wrote.
