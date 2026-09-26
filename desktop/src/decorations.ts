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
//! leaves the traffic lights inset at the top-left — and Windows reads it exactly
//! as Linux does. A Mac ignores the two colours beside it; where its lights go it
//! is told instead, and that is [`TRAFFIC`] to begin with and [`lights`]
//! afterwards.
//!
//! **A Mac's lights are moved rather than painted** (ADR-0020, Set 847 Q11, Set
//! 889 Q4a). Left where `"hidden"` puts them they stand over the pane chrome's
//! padding, in nothing the page drew; what is wanted is the head's own first row,
//! left of the Wordmark. So the same push that recolours the overlay on the other
//! two platforms moves the buttons here — `setWindowButtonPosition`, given a
//! point [`lights`] works out of the head that arrived. Which is why the band is
//! not the only measurement that crosses: the band is how tall the whole strip
//! stands, and the row the lights are centred in is a shorter thing inside it, so
//! [`Head.middle`] is the page saying where that row's middle is.
//!
//! **The four values below are where the window starts rather than where it
//! stays.** The page is what knows what its heads are drawn in, how tall their
//! band actually stands and where the row inside it is, and it pushes all four
//! over the bridge — on load, and again at every flip of the colour scheme. What
//! is written here is the light scheme's paper and ink, and the band and the
//! row's middle at a sixteen-pixel root, so that the window this leaves behind
//! is already right on a light desktop and wrong nowhere for longer than a page
//! takes to load.
//!
//! **So the same four values are what a push carries**, and that is why what
//! arrives over the bridge is read here too: [`Head`] is the shape, [`worn`] is
//! it checked, [`overlay`] is it in Electron's own three words, [`lights`] is the
//! point a Mac's buttons are moved to, and [`overlaid`] and [`buttoned`] are
//! which of the two this platform has. The calls that wear it are `main.ts`'s,
//! there being a window in that one.

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
/// **Which is one of the two that is wrong on somebody's machine**: a human who
/// has told their browser to draw text larger has a taller head, and no constant
/// compiled in here could know it. So the page adds the same rules up against the
/// rem its own browser is drawing and pushes what comes out — `band` in
/// `web/src/head.ts`, which is this sum said where the rem can be asked for. This
/// is the value the window opens at in the moment before it is.
export const BAND = 75;

/// How far down the window the middle of the head's *first row* sits, in pixels.
///
/// The other measurement written in rem, and the other one the page redoes for
/// itself — `middle` in `web/src/head.ts`. A shorter sum than the band's, out of
/// the front of the same rules: the chrome's rem and a quarter less the rem the
/// head hangs back up into it, the head's rem above its row, and half the row.
/// About thirty-nine pixels where a rem is sixteen.
///
/// **Which is not the middle of the band**, and that is the whole reason it is a
/// value of its own. The band carries the head's rem *below* the row as well and
/// a quarter-rem of chrome above it, so its own middle sits a couple of pixels
/// higher than the row's — and the row is what a head's title and controls stand
/// in, so the row is what the traffic lights are centred in.
export const MIDDLE = 39;

/// How tall the traffic lights stand, in pixels — which is what the point they
/// are moved to has to be worked out against, the point being their top-left
/// corner rather than their middle.
///
/// **Measured on a Mac rather than reasoned about** (macOS 15 on Apple silicon,
/// Electron 43): each button reports a sixteen-by-sixteen frame, at twenty points
/// of spacing, whatever the window's own scale — the buttons are the platform's
/// furniture, drawn in points, so this is not a length that follows the page's
/// rem the way everything above it does.
export const LIGHTS = 16;

/// And how far in from the left edge of the window they sit.
///
/// macOS's own inset, kept: the lights belong to the window's corner rather than
/// to the page's column, and twenty points is where the platform draws them. It
/// is also the chrome's rem and a quarter at a sixteen-pixel root, so at the size
/// the viewer is written at they line up with where a pane's own content begins.
///
/// **Not the page's to say, unlike the row they are centred in.** How far *down*
/// they go is a question about a head drawn in rem and so the page's; how far in
/// from the edge is a question about the platform's furniture, and answering it
/// from the page would be a Mac's lights moving because somebody's text got
/// bigger.
export const INSET = 20;

/// What the head is drawn in, how tall its band stands, and where the row inside
/// it is: the four values the window is painted, sized and positioned from.
///
/// The page's own words rather than Electron's, because these are the page's own
/// values — the paper a head is drawn on, the ink its title and its controls are
/// drawn in, the band it stands in and the middle of its first row. [`overlay`]
/// and [`lights`] say them the way a window takes them, and they are the one
/// place the two vocabularies meet.
export interface Head {
  /// The ground the head is drawn on, as `#rrggbb`.
  readonly paper: string;

  /// And the ink its marks are in, the same way — which is what the overlay
  /// draws its own symbols in.
  readonly ink: string;

  /// How tall the band stands, in whole pixels.
  readonly band: number;

  /// And how far down the window the middle of its first row sits, in whole
  /// pixels — what a Mac's traffic lights are centred on, and nothing the other
  /// two platforms have any use for.
  readonly middle: number;
}

/// What the window opens wearing, before the page has said anything: the light
/// scheme's two colours, and the band and the row's middle at a sixteen-pixel
/// root.
export const OPENS: Head = { paper: PAPER, ink: INK, band: BAND, middle: MIDDLE };

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

/// And where a Mac's traffic lights go, given the head the page pushed: the point
/// `setWindowButtonPosition` and `trafficLightPosition` are both said in.
///
/// **The top-left corner of the buttons rather than their middle**, which is the
/// one thing about that point worth writing down — measured on macOS 15 under
/// Electron 43, where a `y` of thirty put a sixteen-pixel button's top thirty
/// pixels down the window and a `y` of sixty put it at sixty. So the middle the
/// page pushed is where the lights are *centred*, and half a light is the
/// difference between that and where they are *put*.
///
/// **And never above the window's own top edge.** A band the page says is a few
/// pixels tall would put the lights at a negative offset, which is three controls
/// half off the window — so the top of the window is as far up as this goes. The
/// inset is [`INSET`]'s and does not move: what the page says is how far down a
/// head stands, and it has no opinion about the platform's own corner.
export function lights(head: Head): { x: number; y: number } {
  return { x: INSET, y: Math.max(0, head.middle - LIGHTS / 2) };
}

/// What the window is made with in place of a title bar.
///
/// Spread into the `BrowserWindow` options rather than passed as one of them —
/// they are options that only mean anything together, and a window given the
/// style without the overlay is a bare strip with nothing in it.
///
/// **And no platform branch, still.** `trafficLightPosition` is a Mac's alone and
/// the overlay's colours are the other two platforms', and each is ignored where
/// it means nothing — so what a window is made with is one value on every
/// platform, and which half of it lands is the platform's own affair.
export const DECORATIONS = {
  titleBarStyle: "hidden",
  titleBarOverlay: overlay(OPENS),
  trafficLightPosition: lights(OPENS),
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
/// **And the two measurements are rounded rather than refused** — see [`whole`],
/// which is that rule said once for both of them: a window is told either of them
/// in whole pixels, and the page's own arithmetic lands on a fraction as readily
/// as on an integer.
///
/// **All four or nothing**, the same way the page pushes all four: a head whose
/// row's middle did not arrive is a head the lights cannot be centred from, and
/// the window a Mac would be left with is one whose buttons stayed where the last
/// push put them rather than one with buttons in the wrong place.
export function worn(pushed: unknown): Head | undefined {
  if (typeof pushed !== "object" || pushed === null) {
    return undefined;
  }

  const { paper, ink, band, middle } = pushed as Record<string, unknown>;

  if (typeof paper !== "string" || !COLOUR.test(paper)) {
    return undefined;
  }

  if (typeof ink !== "string" || !COLOUR.test(ink)) {
    return undefined;
  }

  const tall = whole(band);
  const down = whole(middle);

  if (tall === undefined || down === undefined) {
    return undefined;
  }

  return { paper, ink, band: tall, middle: down };
}

/// One of the two measurements a push carries, as whole pixels — and `undefined`
/// where what arrived is no measurement at all.
///
/// Both of them are lengths a window is told in whole pixels, and both are the
/// page's arithmetic against whatever a rem is on that machine, so both land on a
/// fraction as readily as on an integer and both are rounded rather than refused
/// for it. What has to be true of either is that it is somewhere above nothing.
function whole(measured: unknown): number | undefined {
  if (typeof measured !== "number" || !Number.isFinite(measured)) {
    return undefined;
  }

  const pixels = Math.round(measured);

  return pixels > 0 ? pixels : undefined;
}

/// Whether this platform has an overlay for a push to recolour.
///
/// **A Mac has traffic lights rather than an overlay.** `setTitleBarOverlay`
/// answers *"Titlebar overlay is not enabled"* with a throw there — the same
/// call, on the platform the overlay was never for — so what a push comes to on a
/// Mac is [`buttoned`]'s arm instead: the lights moved rather than the strip
/// repainted. The page has no platform branch in it and says what it is drawn in
/// wherever it is drawn; this and its neighbour are where that one push becomes
/// two platforms' worth of answer.
export function overlaid(platform: NodeJS.Platform): boolean {
  return platform !== "darwin";
}

/// And whether it has buttons for a push to move instead, which is a Mac and
/// nowhere else.
///
/// `setWindowButtonPosition` is `darwin`'s alone: the other two platforms draw
/// the controls overlay [`overlaid`] answers for, at the corner the platform puts
/// it at and nowhere the app could move it to. Said as its own question rather
/// than as the other one's `else`, so that a platform with neither — there is
/// none today — is a push that does nothing rather than a call that throws.
export function buttoned(platform: NodeJS.Platform): boolean {
  return platform === "darwin";
}
