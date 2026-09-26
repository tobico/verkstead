# 03. The frame keeps the outermost heads clear

## What to build

The page reads the overlay's rectangle, works out how much room the controls
take at each edge, and hands the frame two insets. The frame pads its leftmost
and rightmost heads by them, so nothing in a head ever sits under the platform's
controls — in three panes, in two, and in the one pane a narrow window shows,
which is both edges at once.

**The rectangle is the page's area, not the controls'.**
`navigator.windowControlsOverlay.getTitlebarAreaRect()` answers with what the
*page* is left: measured during planning it was `{x: 0, y: 0, width: 909,
height: 44}` in a window whose `innerWidth` was 1006. So the right inset is
`innerWidth - (x + width)` and the left inset is `x` — which is what puts the
Mac's traffic lights and Windows' and Linux's controls through the same
arithmetic without a platform branch. Where there is no
`navigator.windowControlsOverlay` at all — every browser, every phone — both
insets are zero and nothing moves.

**Read again on `geometrychange`, which is not an optimisation.** The rectangle
at load can disagree with the window it is in: on COSMIC the first reading gave
a titlebar area wider than `innerWidth`, and the `geometrychange` that followed
a moment later was the true one. A page that read it once would pad by a stale
number on every launch. The event also carries a maximise, an unmaximise and a
resize, which are the three moments the inset actually changes.

**The frame pads, not the pane.** Which head is outermost is a fact about the
layout rather than about any pane — the sidebar is leftmost where there is a
list to pick from and the details pane is rightmost always, except that a narrow
window shows one pane and it is both, and the compose page with nothing to list
stands one pane across the whole window. All of those are distinctions the frame
already makes for its columns and its dividers, so the padding is written where
they are, keyed off the same layout the frame already knows it is in. The insets
arrive as variables on the frame, the way its column widths already do.

**The Wordmark's Mac inset is written now and proven in stage 06.** On a Mac the
traffic lights sit at the top-left, which is where the Wordmark is — so the left
inset is the one that does work there, and the platform the bridge reports is
what says whether to expect one. There is no Mac here to see it on, which is
what stage 06 is for; what this task owes is that the arithmetic is right and
the suite says so.

## Acceptance criteria

- [ ] No control in the details head sits under the overlay, with three panes
      up, with two, and with the single pane a narrow window shows.
- [ ] The sidebar's head leaves room on the left when the bridge says the
      platform is a Mac, and does not otherwise.
- [ ] Both insets are zero where there is no overlay, and the page in a browser
      is pixel-for-pixel what it was.
- [ ] The viewer's suite covers the arithmetic against a stub bridge and a
      stubbed rectangle — an overlay on the right, one on the left, and none at
      all — and covers the frame reading a later `geometrychange`.
