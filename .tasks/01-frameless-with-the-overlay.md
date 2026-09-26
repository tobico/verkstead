# 01. Frameless, with the overlay on

## What to build

The window is made with no title bar and the platform's controls overlay on, at
about the head's band and in the light scheme's colours. It draws with no title
bar of its own, the three controls work, and a double-click on the top strip
maximises it.

**`titleBarStyle: 'hidden'`, not `frame: false`** — this was probed on the
pinned Electron (43.1.0) while the stage was planned, and the two are not
interchangeable on Linux. `frame: false` with `titleBarOverlay` beside it gives
a window whose page reports `navigator.windowControlsOverlay.visible === false`,
a titlebar-area rectangle of zeroes, and a `setTitleBarOverlay` that throws
*"Titlebar overlay is not enabled"*. `titleBarStyle: 'hidden'` with the same
`titleBarOverlay` gives a working overlay: the controls draw at the top-right,
the rectangle is real, it follows a resize, and the colours can be changed while
the window is open. It is also the one option a Mac takes — where it hides the
bar and leaves the traffic lights — so there is no platform branch here at all.

**The colours and the height are a starting constant in this task and stop
being one in task 04.** The page is what knows them, and what it pushes is what
the overlay ends up wearing. What goes in here is the light scheme's paper and
ink and a height about the head's band, so that the window this task leaves
behind is already right on a light desktop.

**The options go somewhere the suite can reach.** The module holding the
`BrowserWindow` cannot be run under vitest and the lint wall names it, which is
why everything the window *decides* already lives beside it rather than in it —
where it opens, what a close means, which navigations are its own. The overlay's
options are another of those: a value some module works out and the window is
handed, so the suite can pin the style, the height and the two colours without a
window.

**The menu bar is still hidden** on Windows and Linux with its shortcuts kept,
which is stage 02's and is not touched here; a Mac keeps its application menu.
And a double-click on a drag region maximises with nothing written for it —
Electron does that itself on Linux, proven during planning — so the criterion
below is a check rather than a feature.

## Acceptance criteria

- [ ] The window draws with no title bar, and minimise, maximise and close all
      work from the overlay.
- [ ] A double-click on the top strip of the page maximises the window, and
      another restores it.
- [ ] The desktop suite pins the title-bar style, the overlay's height and its
      two colours, without constructing a window.
- [ ] The app comes up and loads the workbench as it did before, the menu bar
      still hidden.
