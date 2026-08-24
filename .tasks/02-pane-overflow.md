# 02. Fix the page growing under the panes

## What to build

With the detail pane open, the document sometimes grows taller than the
viewport: blank space appears under the three panes, and once a pane is
scrolled to its bottom the browser scrolls the whole page, pushing the
workbench off screen.

Two diagnosed causes, both to address:

1. The detail pane holding a Screen is given a hard `100dvh` height of its
   own *inside* the `100dvh` workbench grid, and nothing between it and the
   body clips overflow — so any mismatch between the two viewport-height
   resolutions becomes document height. Let that pane take its height from
   its grid row instead of the viewport, and/or clip overflow on the
   workbench container at desktop widths. The rule also applies outside the
   wide-screen media query today, where the workbench has no height at all —
   re-ground it.
2. No `overscroll-behavior` anywhere: a pane at its scroll end chains the
   scroll to the document. Contain overscroll on the scrolling panes (and
   the terminal host).

Keep the phone-first behaviour intact: below the wide breakpoint the page
itself scrolls by design.

## Acceptance criteria

- [ ] No blank space below the panes with a detail pane (including the
      Screen) open at desktop widths
- [ ] Scrolling past a pane's bottom or top stays in the pane; the page
      never scrolls at desktop widths
- [ ] Narrow-screen single-pane scrolling behaves as before
