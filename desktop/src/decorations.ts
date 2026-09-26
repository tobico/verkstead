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
//! their band actually stands, and it pushes all three over the bridge — on
//! load, and again at every flip of the colour scheme. What is written here is
//! the light scheme's paper and ink and the band at a sixteen-pixel root, so
//! that the window this leaves behind is already right on a light desktop and
//! wrong nowhere for longer than a page takes to load.
//!
//! **So the same three values are what a push carries**, and that is why what
//! arrives over the bridge is read here too: [`Head`] is the shape, [`worn`] is
//! it checked, [`overlay`] is it in Electron's own three words, and [`overlaid`]
//! is whether this platform has an overlay for any of it to reach. The call that
//! wears it is `main.ts`'s, there being a window in that one.

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
/// no constant compiled in here could know it. So the page adds the same rules
/// up against the rem its own browser is drawing and pushes what comes out —
/// `band` in `web/src/head.ts`, which is this sum said where the rem can be
/// asked for. This is the value the window opens at in the moment before it is.
export const BAND = 75;

/// What the head is drawn in and how tall its band stands: the three values the
/// overlay is painted and sized from.
///
/// The page's own words rather than Electron's, because these are the page's own
/// values — the paper a head is drawn on, the ink its title and its controls are
/// drawn in, and the band it stands in. [`overlay`] says them the way a window
/// takes them, and it is the one place the two vocabularies meet.
export interface Head {
  /// The ground the head is drawn on, as `#rrggbb`.
  readonly paper: string;

  /// And the ink its marks are in, the same way — which is what the overlay
  /// draws its own symbols in.
  readonly ink: string;

  /// How tall the band stands, in whole pixels.
  readonly band: number;
}

/// What the window opens wearing, before the page has said anything: the light
/// scheme's two colours, and the band at a sixteen-pixel root.
export const OPENS: Head = { paper: PAPER, ink: INK, band: BAND };

/// A colour the way Electron reads one, which is the way the page pushes one.
const COLOUR = /^#[0-9a-f]{6}$/i;

/// The head as a window wears it: Electron's three words for the page's three
/// values.
///
/// Here rather than at either end of the bridge, so that the overlay the window
/// is made with and the recolour a push comes to are one translation said once.
/// A `color` that is a paper and a `symbolColor` that is an ink is the whole of
/// the difference between the two vocabularies, and it is worth exactly this.
export function overlay(head: Head): {
  color: string;
  symbolColor: string;
  height: number;
} {
  return { color: head.paper, symbolColor: head.ink, height: head.band };
}

/// What the window is made with in place of a title bar.
///
/// Spread into the `BrowserWindow` options rather than passed as one of them —
/// it is two options that only mean anything together, and a window given the
/// style without the overlay is a bare strip with nothing in it.
export const DECORATIONS = {
  titleBarStyle: "hidden",
  titleBarOverlay: overlay(OPENS),
} as const;

/// What the page pushed, where what arrived is a head — and `undefined` where it
/// is anything else.
///
/// Checked rather than cast, for the reason a set is — see
/// [`changed`](./settings.js): a bridge is not to be trusted with a shape just
/// because the window is the app's own, and what an unchecked push reaches is a
/// call that throws at a colour it cannot parse. A push that is not a head
/// changes nothing, which is an overlay left exactly as it was.
///
/// **And the band is rounded rather than refused.** Electron measures the
/// overlay in whole pixels, and the page's own arithmetic — a count of rem
/// against whatever a rem is on that machine — lands on a fraction as readily as
/// on an integer. Being a number of pixels at all is what has to be true of it;
/// being a round one is this function's to make.
export function worn(pushed: unknown): Head | undefined {
  if (typeof pushed !== "object" || pushed === null) {
    return undefined;
  }

  const { paper, ink, band } = pushed as Record<string, unknown>;

  if (typeof paper !== "string" || !COLOUR.test(paper)) {
    return undefined;
  }

  if (typeof ink !== "string" || !COLOUR.test(ink)) {
    return undefined;
  }

  if (typeof band !== "number" || !Number.isFinite(band) || band <= 0) {
    return undefined;
  }

  return { paper, ink, band: Math.round(band) };
}

/// Whether this platform has an overlay for a push to reach at all.
///
/// **A Mac has traffic lights rather than an overlay.** `titleBarStyle:
/// "hidden"` leaves them inset at the top-left there (ADR-0020), and
/// `setTitleBarOverlay` answers *"Titlebar overlay is not enabled"* with a throw
/// — the same call, on the platform the overlay was never for. So a push from a
/// Mac's page changes nothing: the page has no platform branch in it and says
/// what it is drawn in wherever it is drawn, and this is where the answer to it
/// is nothing at all rather than an error crossing back over the bridge.
export function overlaid(platform: NodeJS.Platform): boolean {
  return platform !== "darwin";
}
