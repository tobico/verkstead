# 04. The traffic lights in the head's first row

## What to build

The window has no title bar, and on a Mac `titleBarStyle: "hidden"` leaves the
traffic lights where macOS puts them — inset at the top-left of the window,
which is over the pane chrome's padding rather than in anything the page drew.
ADR-0020 and Set 847 Q11 want them **in the head's first row, left of the
Wordmark**, with the sidebar's head leaving them room.

**The app moves the buttons on every head the page pushes** (Set 889 Q4a). How
tall a head stands is the page's to know and not the app's to assume: it is
written in rem, so a human who has told their browser to draw text larger has a
taller head, which is why `head.ts` already adds those rules up against its own
resolved rem and pushes the result over the bridge, and why `main.ts` already
answers that push. On a Mac the answer is silence today — `setTitleBarOverlay`
is Windows' and Linux's, and `overlaid()` is what says so. What it becomes is
`setWindowButtonPosition`, which is `darwin`'s, given a point worked out from
the head that was pushed. So the lights follow a larger text size the way the
overlay's height does, and neither platform's arm is a constant compiled into
the app.

**What the page has to push for that to be possible.** `band` says how tall the
whole band is; it does not say where the row inside it sits, and the row is what
the lights are centred in — the band also carries the chrome's padding above the
row and the head's own rem below it. Whether `Head` grows a field for the row,
or the app derives the point from the band and the same rules, is this task's to
settle. What it may not do is write the head's stylesheet rules into the app a
second time: that duplication is the thing `head.ts` exists to avoid, and a
second copy would be wrong on exactly the machine the first one was written for.

`trafficLightPosition` in the window's own options is where the lights *start*,
before the page has said anything — the same relationship the overlay's opening
colours have to the push that replaces them.

**The room they take** (Set 889 Q4b). `controls.ts` turns
`getTitlebarAreaRect()` into `--controls-left` and `--controls-right` on the
frame with no platform branch in it, and the stylesheet pads the outermost pane
heads by them — which stage 04 wrote expecting a Mac's lights to come out as a
left inset. Electron's own documentation for `titleBarOverlay` says it enables
the Window Controls Overlay APIs on a Mac as well, so that may already work and
there may be nothing to do. **Measure it first.** Where the overlay answers
nothing there, the decision in force is that the app pushes the inset the lights
take over the bridge and the frame reads it exactly as it reads the overlay's —
one more thing the app says and the page wears, rather than a constant in the
stylesheet. And the lights move whenever the head does, so whatever carries the
inset has to be said again then.

## Acceptance criteria

- [ ] The lights sit in the head's first row, vertically centred and clear to
      the left of the Wordmark, at the browser's default text size and at a
      larger one.
- [ ] Nothing the page draws is underneath them at one, two or three panes, and
      the sidebar's head keeps its distance across a resize and a maximise.
- [ ] The desktop suite pins whatever the app works the point out from and the
      viewer suite pins whatever the frame reserves, both as ordinary unit tests
      on this Linux runner; a browser and a phone are padded by nothing, as
      before.
