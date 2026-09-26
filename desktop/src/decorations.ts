//! The window's title bar taken away, and the platform's own controls left
//! standing where it was.
//!
//! A value rather than anything that happens: this is what the `BrowserWindow`
//! in [`window.ts`](./window.js) is made with. It is here rather than in there
//! for the reason everything the window decides is — that module holds a window
//! and so cannot be run by vitest at all, and the lint wall in
//! `eslint.config.js` names it. What is left in the window is the window.
//!
//! **`titleBarStyle: "hidden"` rather than `frame: false`**, and the two are not
//! interchangeable on Linux. Probed on the pinned Electron while this stage was
//! planned: `frame: false` with a `titleBarOverlay` beside it gives a window
//! whose page reports `navigator.windowControlsOverlay.visible === false`, a
//! titlebar-area rectangle of zeroes, and a `setTitleBarOverlay` that throws
//! *"Titlebar overlay is not enabled"*. The same `titleBarOverlay` under
//! `titleBarStyle: "hidden"` gives a working overlay: the controls draw at the
//! top-right, the rectangle is real, it follows a resize, and the colours can be
//! changed while the window is open.
//!
//! **And no platform branch, because there is nothing to branch on.**
//! `"hidden"` is the one option a Mac takes as well — there it hides the bar and
//! leaves the traffic lights inset at the top-left, which is what ADR-0020 asks
//! for on that platform — and Windows reads it exactly as Linux does. A Mac
//! ignores the overlay beside it; the two colours and the height are the other
//! two platforms' alone.
//!
//! **The three values below are where the overlay starts rather than where it
//! stays.** The page is what knows what its heads are drawn in and how tall
//! their band actually stands, and from the fourth task of this stage it pushes
//! all three over the bridge — on load, and again at every flip of the colour
//! scheme. What is written here is the light scheme's paper and ink and the
//! band at a sixteen-pixel root, so that the window this leaves behind is
//! already right on a light desktop and wrong nowhere for longer than a page
//! takes to load.

/// The paper the head is drawn on, in the light scheme — `--paper` in
/// `web/src/styles/base.css`, which is what `.paneChrome` puts behind every
/// pane's header.
///
/// A `#rrggbb` string because that is what Electron parses; the page pushes the
/// colour it has actually resolved in the same notation.
export const PAPER = "#faf8f5";

/// And the ink the head's marks are in — `--ink` in the same file, which is
/// what the title and the pane's own controls are drawn in. The overlay draws
/// its three symbols in it.
export const INK = "#1c1a17";

/// How tall the overlay stands, in pixels.
///
/// The head's band, added up out of the rules that draw it: a rem and a quarter
/// of the pane chrome's own top padding, less the rem the head hangs back up
/// into it, plus the head's rem above the row and its rem below, and a row as
/// tall as the icon buttons standing in it. About seventy-five pixels where a
/// rem is sixteen.
///
/// **Which is the one of the three that is wrong on somebody's machine**: a
/// human who has told their browser to draw text larger has a taller head, and
/// no constant compiled in here could know it. Stage 04's fourth task is where
/// the page measures the band it has actually drawn and says so; this is the
/// value the window opens at in the moment before it does.
export const BAND = 75;

/// What the window is made with in place of a title bar.
///
/// Spread into the `BrowserWindow` options rather than passed as one of them —
/// it is two options that only mean anything together, and a window given the
/// style without the overlay is a bare strip with nothing in it.
export const DECORATIONS = {
  titleBarStyle: "hidden",
  titleBarOverlay: {
    color: PAPER,
    symbolColor: INK,
    height: BAND,
  },
} as const;
