# 05. The Screen fills the pane

## What to build

Make the Screen view take the available page height the way it already takes
the available width. Today the browser measures how many columns fit in the
pane and sends them up the live socket, but echoes the server's own row count
back — so the grid keeps whatever height the server says, and the pane
scrolls or leaves it short.

For a live session, measure rows the way columns are measured and send both:
the session's terminal redraws at that height, the repaint that comes back is
the answer, and **the latest window wins for every watcher** — exactly the
rule width follows now, extended to the second dimension. The pane stops
scrolling under a live Screen: the terminal takes the height left below the
header and switcher, with the note or Hold bar under it still in view.

An ended session's grid is fixed — nothing can resize it — so it is shown at
its own height and **scrolls inside the pane** when it is taller, rather than
scrolling the page. The Transcript side of the switcher keeps its ordinary
page-column scrolling; only the Screen view is height-bound.

The Screen component's own reasoning for sending columns alone is written in
its module documentation; that argument changes with this task and the
documentation changes with it.

## Acceptance criteria

- [ ] A live Screen fills the pane's height with nothing scrolling under it,
      and resizing the window resizes the session's terminal rows as well as
      its columns.
- [ ] Two watchers with panes of different heights converge on the most
      recently resized one, as width does today.
- [ ] An ended session's grid draws at its own size and scrolls inside the
      pane when taller than it.
- [ ] Switching back to Transcript restores ordinary pane scrolling.
