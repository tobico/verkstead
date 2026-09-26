# 04. The head's colours and height over the bridge

## What to build

The page pushes the head's ground colour, its symbol colour and how tall its
band actually stands, on load and again whenever the colour scheme flips. The
main process wears them: the overlay is recoloured and resized while the window
is open, with no restart.

**The colours come from the page** (ADR-0020, Set 847 Q11b). Two fixed colours
were rejected, and the reason is the whole of this task: the app has a light
scheme and a dark one, the head is drawn on the paper and its marks in the ink,
and an overlay that did not follow would be a strip of the wrong colour welded
to the corner of the window. So the page reads what it is actually drawn in and
says so, and the app has no opinion about either value.

**The height comes from the page too** — settled with the human while planning.
The head's band is written in `rem`: a rem and a quarter of chrome padding, the
head's own rem above and below, and a row as tall as the icon buttons standing
in it, which comes to about 75px at a 16px root. The overlay's height is a pixel
integer the main process sets. A constant in the app would be right on one
machine and wrong on the next — a human who has told their browser to draw text
larger has a taller head — so the page measures the band it has actually drawn
and pushes it with the colours. One channel, three values, one moment.

**A flip is `prefers-color-scheme` and nothing else.** There is no theme switch
in this app; the scheme is the machine's, and two places already watch it the
way this one has to. That is the pattern to follow rather than a mechanism to
invent.

**A channel of its own on the bridge**, beside the five that are there. The
shape is written twice on purpose — the preload's side is the authority and the
page's side is it said again, because the two packages are built apart — so this
one is pinned on both sides by the two suites that already pin the other five,
and a shape that drifted would be caught there.

**And it does nothing on a Mac.** `setTitleBarOverlay` is Windows' and Linux's:
a Mac has traffic lights rather than an overlay, and the same call throws
*"Titlebar overlay is not enabled"* wherever there is no overlay to recolour. So
the main process answers the push by doing nothing where there is nothing to
recolour, rather than by throwing across the bridge. A browser has no bridge at
all, so it pushes nothing and this whole file is absent from what it runs.

## Acceptance criteria

- [ ] Flipping the machine's colour scheme while the app is open recolours the
      overlay to match the head, with no restart.
- [ ] The overlay stands exactly as tall as the head's band, and follows it when
      the root font size is not 16px.
- [ ] A push on a platform with no overlay changes nothing and throws nothing.
- [ ] Both suites pin the new channel's shape, as they pin the other five, and
      the viewer's suite covers a page with no bridge pushing nothing.
