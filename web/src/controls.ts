//! The platform's own window controls, and how much room they take at each edge
//! of the window.
//!
//! The app's window has no title bar (ADR-0020), so the platform draws its three
//! controls over the top corner of the page itself: the controls overlay at the
//! top-right on Windows and Linux, the traffic lights at the top-left on a Mac.
//! Whatever the page puts in that corner is underneath them, which is a control
//! nobody can press — so the page has to know how much of each edge is not its
//! own, and the frame is what pads its outermost heads by it (`Panes.tsx`).
//!
//! **The rectangle is the page's area rather than the controls'.**
//! `getTitlebarAreaRect()` answers with what is left *for the page* across the
//! top of the window: measured while this stage was planned it was
//! `{ x: 0, y: 0, width: 909, height: 44 }` in a window whose `innerWidth` was
//! 1006. So what the controls took is the two slivers either side of it — `x` on
//! the left, and whatever lies beyond its far edge on the right. One sum for
//! every platform and no branch in it: a Mac's traffic lights come out as a left
//! inset and Windows' and Linux's controls as a right one, because that is where
//! each platform put them, and the page never has to ask which it is on.
//!
//! **Read again on `geometrychange`, which is not an optimisation.** The
//! rectangle at load can disagree with the window it is in — on COSMIC the first
//! reading gave a titlebar area wider than `innerWidth`, and the
//! `geometrychange` a moment later was the true one — so a page that read it
//! once would pad by a stale number on every launch. The same event carries a
//! maximise, an unmaximise and a resize, which are the three moments an inset
//! actually changes.
//!
//! **And none of it reaches a browser.** `navigator.windowControlsOverlay` is
//! there to be asked or it is not, and where it is there it says whether it is
//! `visible`: a browser, a phone and a window that was never made frameless all
//! answer [`CLEAR`], which is nothing padded and nothing moved. That is the whole
//! guard — the frame writes no variable it has no inset for, so the document a
//! browser is served is the document it was.

import { createSignal, onCleanup, type Accessor } from "solid-js";

/// How much room the controls take at each edge of the window, in CSS pixels.
///
/// Both at once rather than one and a platform, because a narrow window's single
/// pane stands at both edges and has to be clear of whichever of them the
/// controls are at.
export interface Insets {
  readonly left: number;
  readonly right: number;
}

/// Both edges the page's own, which is every browser and every phone.
export const CLEAR: Insets = { left: 0, right: 0 };

/// The strip across the top of the window the page is left — as much of
/// `getTitlebarAreaRect()`'s answer as the insets are worked out from.
///
/// The height is the overlay's own and nothing here reads it: how tall the band
/// is is something the page *tells* the app rather than something it asks, the
/// band being as tall as the head the page drew.
export interface Area {
  readonly x: number;
  readonly width: number;
}

/// What the controls take from a window `across` pixels wide, given the area the
/// page was left.
///
/// The arithmetic on its own, so that the three shapes it has to answer for can
/// be asked of it without a window: an overlay at the right, one at the left, and
/// no overlay at all.
///
/// Nothing negative comes out of it. A titlebar area wider than the window it is
/// in is what COSMIC answers before its first `geometrychange`, and an inset of
/// less than nothing is a head padded backwards — so what is not a positive
/// number of pixels is no inset.
export function insets(area: Area | undefined, across: number): Insets {
  if (area === undefined || !(across > 0)) {
    return CLEAR;
  }

  return { left: room(area.x), right: room(across - (area.x + area.width)) };
}

/// How much room the controls are taking now, and again whenever they move.
///
/// Read at the moment the frame is built and then followed: the rectangle at load
/// is not always the true one, and a maximise, an unmaximise and a resize each
/// change what the page is left. A window with no overlay to ask has nothing to
/// follow either, so it stands at [`CLEAR`] for as long as it is open.
export function controls(): Accessor<Insets> {
  const overlay = reached();
  const [taken, setTaken] = createSignal(taking(overlay));

  if (overlay !== null) {
    const moved = () => setTaken(taking(overlay));

    overlay.addEventListener?.("geometrychange", moved);
    onCleanup(() => overlay.removeEventListener?.("geometrychange", moved));
  }

  return taken;
}

/// What the frame carries the insets as, for the rules that pad its outermost
/// heads by them.
///
/// Variables on the frame, the way its column widths already are — and an edge
/// the controls take nothing from is left unsaid rather than written as nought:
/// the stylesheet has the nought behind each name, so a browser's frame carries
/// exactly the style attribute it carried before any of this.
export function reserved(taken: Insets): Record<string, string> {
  return {
    ...(taken.left > 0 ? { "--controls-left": `${taken.left}px` } : {}),
    ...(taken.right > 0 ? { "--controls-right": `${taken.right}px` } : {}),
  };
}

/// What the window's controls take, as the overlay in front of them answers.
///
/// An overlay that says it is not `visible` is a window whose controls are not
/// drawn over the page at all — where the rectangle is a row of zeroes, and
/// where reading it as an inset would pad the whole window away.
function taking(overlay: Overlay | null): Insets {
  if (overlay === null || !overlay.visible) {
    return CLEAR;
  }

  return insets(overlay.getTitlebarAreaRect(), window.innerWidth);
}

/// As much of `navigator.windowControlsOverlay` as this asks for.
interface Overlay {
  readonly visible: boolean;
  getTitlebarAreaRect(): Area;
  addEventListener?(kind: "geometrychange", heard: () => void): void;
  removeEventListener?(kind: "geometrychange", heard: () => void): void;
}

/// The overlay this page is drawn under, or `null` where there is none.
///
/// Checked for its shape rather than cast into it, the way the app's own bridge
/// is (`settings/bridge.ts`): what is on `navigator` under this name is a
/// browser's to decide, and *something is there* is not the same as *the controls
/// are over the page*.
function reached(): Overlay | null {
  const held = (navigator as { windowControlsOverlay?: unknown })
    .windowControlsOverlay;

  if (typeof held !== "object" || held === null) {
    return null;
  }

  const asked = held as Record<string, unknown>;

  return typeof asked.visible === "boolean" &&
    typeof asked.getTitlebarAreaRect === "function"
    ? (held as Overlay)
    : null;
}

/// One edge's worth of room: what is left of a measurement that has to be a
/// positive number of pixels to mean anything.
function room(pixels: number): number {
  return Number.isFinite(pixels) && pixels > 0 ? pixels : 0;
}
