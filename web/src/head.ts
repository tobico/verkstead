//! What the head is drawn in and how tall its band stands, told to the app.
//!
//! The app's window has no title bar, so the platform draws its own controls on a
//! strip of the top corner of the page itself (ADR-0020) — and that strip is the
//! app's to paint rather than the page's. Which is the whole of why anything
//! crosses here. **Two fixed colours in the app were rejected** (Set 847 Q11b):
//! the viewer has a light scheme and a dark one, a head is drawn on the paper of
//! whichever the machine is in with its marks in the ink, and an overlay that did
//! not follow would be a strip of the wrong colour welded to the corner of the
//! window. So the page reads what it is actually drawn in and says so, and the
//! app has no opinion about either value.
//!
//! **And the two measurements are the page's for a neighbouring reason.** The
//! band is written in rem — a rem and a quarter of chrome padding, the head's own
//! rem above its row and below it, and a row as tall as the icon buttons standing
//! in it — while what the overlay is sized in is whole pixels. A constant compiled
//! into the app would be right on one machine and wrong on the next: a human who
//! has told their browser to draw text larger has a taller head. So [`band`] is
//! those same rules added up where the rem can be asked for, and the page pushes
//! what comes out.
//!
//! **And [`middle`] is the shorter sum beside it, which a Mac is what needs.**
//! The lights there are moved into the head's *first row* rather than painted
//! over the whole band (Set 889 Q4a), and the row is not the band: the band
//! carries the chrome's quarter-rem above the row and the head's rem below it as
//! well. So the app is told where the row's middle sits and centres the buttons on
//! it — and the arithmetic stays here, where the rules that draw the head are
//! already added up once, rather than being written into the app a second time.
//!
//! **A flip of the scheme is `prefers-color-scheme` and nothing else.** There is
//! no theme switch in this app — the scheme is the machine's — so the media query
//! is both the question and the only warning that the answer has changed, which
//! is what `set/diagrams.ts` and `workbench/Editor.tsx` already read it as.
//!
//! **And none of it reaches a browser.** The push goes over the preload bridge,
//! which a browser on this machine and a phone on the tailnet do not have — so
//! [`dress`] finds nothing to say anything to and says nothing, exactly as the
//! Desktop section of the settings page draws nothing. There is no branch beyond
//! that one: the same document, drawn the same way, telling nobody.

import { bridge, type Head } from "./settings/bridge";

/// The whole of how the page decides which colours it is in, and so the only
/// notice it gets that they have changed.
const SCHEME = "(prefers-color-scheme: dark)";

/// A rem and a quarter of chrome above the head — `.pane > .paneChrome`'s own top
/// padding in `Panes.module.css`, which is what the record would otherwise slide
/// up against the underside of the title through.
const CHROME = 1.25;

/// Less the rem the head hangs back up into it — `.head`'s negative top margin in
/// `workbench/PaneHead.module.css`, which is the room its controls hang in.
const HANG = 1;

/// The head's own rem above its row, and its rem below — the same rule's padding.
/// The one below is the gap between the header and whatever follows it inside the
/// sticky block, and it is part of the band because it is part of the paper.
const ABOVE = 1;
const BELOW = 1;

/// And the row itself, as tall as the icon buttons standing in it: `0.6rem` of
/// padding either side of a line `1.1rem` tall — `IconButton.module.css`. The
/// tallest thing a head carries, and so the thing the row is the height of.
const ROW = 2.3;

/// How far down the band the row itself starts, in rem: the chrome's padding less
/// the rem the head hangs back up into it, and the head's own rem above the row.
///
/// The front of the same sum the band is, and the half of it that says where the
/// row is rather than how tall the whole strip stands.
const DOWN = CHROME - HANG + ABOVE;

/// What all of that comes to, in rem.
const REMS = DOWN + BELOW + ROW;

/// And the two pixels of the band that are not a rem: the icon button reserves an
/// edge it does not draw — a pixel at the top of the row and a pixel at the
/// bottom — so that it can be given one without growing.
const EDGES = 2;

/// What a rem is where the page cannot be asked, which is every browser's own
/// default and the one this app is written at.
const ROOT = 16;

/// How tall the head's band stands in this browser, in whole pixels.
///
/// The rules that draw it, added up against the rem the page is actually being
/// drawn at — the same reading `Panes.tsx` makes of its own floors, and for the
/// same reason: a length written in rem is a length that follows what the human
/// told their browser, and turning one into pixels is the browser's to be asked
/// about rather than this file's to assume.
///
/// Whole pixels because that is what the overlay is sized in. Exported for the
/// bare drag bar as well, which stands where a head would stand on a page that
/// has none.
export function band(): number {
  return Math.round(REMS * root() + EDGES);
}

/// And how far down the window the middle of the head's *first row* sits, in whole
/// pixels — what a Mac's traffic lights are centred on.
///
/// The row rather than the band, which is the whole of why this is a second
/// measurement: the band is the strip of paper the controls are drawn over, and
/// the row is the line inside it that a head's title and its buttons stand in.
/// Half a rem and change separates the two middles, which is enough to see in
/// three circles beside a wordmark.
///
/// Measured from the top of the window because that is where the band starts: the
/// app has no title bar, so the first pixel of the page is the first pixel of the
/// window, and a point handed to `setWindowButtonPosition` is measured from the
/// same corner.
export function middle(): number {
  const rem = root();

  return Math.round(DOWN * rem + (ROW * rem + EDGES) / 2);
}

/// What the head is drawn in, how tall it stands and where its row is, as the app
/// wears it — or nothing at all where the page cannot say what it is drawn in.
///
/// All four or none of them, there being one push: an overlay given a height and
/// left the colour it opened at is a strip of the light scheme's paper on a dark
/// desktop, which is worse than the strip that is merely the wrong height.
export function worn(): Head | undefined {
  const colours = drawn();

  return colours === undefined ? undefined : { ...colours, band: band(), middle: middle() };
}

/// Tell the app what its head is drawn in, and again at every flip of the colour
/// scheme. Returns what stops watching.
///
/// **Nothing at all outside the app.** The bridge is read once, the way the
/// Desktop section reads it — a preload is there before the document is, so
/// nothing can arrive or leave while a page is open — and a page with none has
/// nobody to tell and nothing to watch for.
export function dress(): () => void {
  const app = bridge();

  if (app === null) {
    return () => {};
  }

  const push = () => {
    const head = worn();

    if (head !== undefined) {
      app.head(head);
    }
  };

  push();

  if (typeof window.matchMedia !== "function") {
    return () => {};
  }

  const scheme = window.matchMedia(SCHEME);

  scheme.addEventListener?.("change", push);

  return () => scheme.removeEventListener?.("change", push);
}

/// The paper the head is drawn on and the ink its marks are in, as the page has
/// actually resolved them.
///
/// **Read off `body` rather than out of `--paper` and `--ink` themselves**, and
/// that is the point of it: a custom property is handed back as the text it was
/// written as, while `background-color` and `color` come back as the browser's own
/// resolved `rgb(…)` — and `body` in `styles/base.css` is drawn in exactly those
/// two tokens, which is also what a head is drawn in. So whatever the scheme has
/// made of them is what this reads, in a notation there is a `#rrggbb` to be had
/// from.
function drawn(): { paper: string; ink: string } | undefined {
  const page = getComputedStyle(document.body);

  const paper = hex(page.backgroundColor);
  const ink = hex(page.color);

  return paper === undefined || ink === undefined ? undefined : { paper, ink };
}

/// The notations three channels can be read out of: `rgb()` and the `rgba()` a
/// colour with an alpha is answered in, in either the comma'd spelling Chromium
/// serializes today or the space-separated one. A computed `background-color` is
/// one of these or it is a colour this cannot turn into `#rrggbb`.
const RGB = /^rgba?\(/i;

/// A resolved colour as the `#rrggbb` Electron parses, or nothing where it is not
/// one.
///
/// Three channels out of the notation the browser answered in — `rgb(250, 248,
/// 245)` today, `rgb(250 248 245)` wherever it has been modernised — and
/// **nothing at all where the colour is transparent**: a page with no stylesheet
/// behind it reports `rgba(0, 0, 0, 0)` for a background nobody set, and an
/// overlay painted from that would be a black strip rather than a head.
///
/// **And nothing at all where the notation is not one of those two**, which is
/// [`RGB`]'s whole job: three numbers pulled out of a string are only three
/// channels if the string was saying channels. `oklch(0.97 0.01 80)` names the
/// same near-white paper and its first three numbers make `#010000`, so a page
/// served by a browser that had modernised its serialization further would push
/// a near-black strip rather than push nothing — which is the one failure this
/// cannot answer for, an overlay being paint on the window rather than a value
/// somebody reads.
function hex(colour: string): string | undefined {
  if (!RGB.test(colour)) {
    return undefined;
  }

  const channels = colour.match(/[\d.]+/g)?.map(Number);

  if (channels === undefined || channels.length < 3) {
    return undefined;
  }

  // Transparent is *unset* rather than a colour, which is what the fourth channel
  // is read for and the only thing it is read for.
  const alpha = channels[3];

  if (alpha !== undefined && !(alpha > 0)) {
    return undefined;
  }

  const rgb = channels.slice(0, 3);

  if (!rgb.every((value) => Number.isFinite(value))) {
    return undefined;
  }

  return `#${rgb.map(channel).join("")}`;
}

/// One channel of it, as the two hex digits a colour is written in.
function channel(value: number): string {
  return Math.max(0, Math.min(255, Math.round(value)))
    .toString(16)
    .padStart(2, "0");
}

/// What a rem is on this page.
///
/// The one part of the band no stylesheet can be told in advance, and the reason
/// the height travels over the bridge at all. Asked of the root element, because
/// that is what a rem is; the app's own sixteen where a browser answers nothing,
/// which is the same fallback `Panes.tsx` keeps.
function root(): number {
  return parseFloat(getComputedStyle(document.documentElement).fontSize) || ROOT;
}
