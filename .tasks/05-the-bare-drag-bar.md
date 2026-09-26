# 05. The bare drag bar

## What to build

A page with no pane head gets a bar of the head's height at the top of it,
draggable and otherwise empty, drawn only inside the app. There are three such
pages as this stage stands:

- **Onboarding**, which is the only page there is while onboarding mode is on.
- **The no-such-page**, which is one line of notice and nothing else.
- **The moment before the verdict lands** — the gate draws nothing at all while
  it is reading whether this machine is set up, and a window that cannot be
  moved for the length of that read is the same hole as the other two. Settled
  with the human while planning (Q4): it gets the bar too.

One component in three places, rather than three bars.

**Only where the bridge is.** A drag region is inert in a browser, so a bar
drawn there would do nothing — but it would still be a bar: a band of the head's
height at the top of the setup page that a phone has no use for. So this one is
drawn on the bridge being there, the way the Desktop section of the settings is,
rather than drawn always and relied on to be harmless.

**The head's height, from the same place task 04 measures it.** The bar stands
where a head would stand and the overlay is as tall as a head, so a bar of some
other height would leave the controls hanging over or under its edge.

## Acceptance criteria

- [ ] The setup page, the no-such-page and the app's first moment can each be
      moved by the top of the window.
- [ ] A browser draws no bar at all on any of the three, and the pages are
      otherwise exactly what they were.
- [ ] The bar is the height of a pane head, so the overlay's controls sit
      within it.
- [ ] The viewer's suite covers a page with no head, with a bridge and without
      one.
