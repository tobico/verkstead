//! Where a scroll actually happens, and whose scroll it was.
//!
//! Two questions the viewer asks in two places and answers the same way both
//! times. Which box moves is not a fact any component knows about itself: the
//! same markup is a column of the page on a phone and the inside of a pane on a
//! desktop, and the stylesheet is what decides which — so the box is found by
//! walking up from what is drawn rather than written down beside it.
//!
//! And whose scroll it was matters wherever a page moves itself: a view that
//! took every scroll for the human's would take its own for one too. The
//! gestures below are the human's own — a wheel, a finger, a key, and the
//! scrollbar being taken hold of — and a page's own scrolling is none of them.
//!
//! [`followBottom`] is the one thing here that is more than a question: a view
//! held at the bottom of a record that is still being written, which is what
//! reading a running session's Transcript is.

import { createEffect, onCleanup, type Accessor } from "solid-js";

/// The box `target` is scrolled inside: the nearest ancestor that scrolls, or
/// `null` where nothing between it and the top of the page does — which is the
/// window, the page itself being what moves then.
///
/// Asked of the layout rather than assumed, because one piece of markup is read
/// in more than one place: a Set is a page of its own and a details pane both,
/// and a pane above the first breakpoint scrolls on its own while the window
/// behind it does not move at all. Scrolling the window there moves nothing.
export function scroller(target: Element): HTMLElement | null {
  for (let at = target.parentElement; at !== null; at = at.parentElement) {
    const how = getComputedStyle(at).overflowY;
    if (how === "auto" || how === "scroll") {
      return at;
    }
  }

  return null;
}

/// The human moving a view themselves: the ways of scrolling that are theirs
/// rather than something the page did to itself.
///
/// A wheel, a finger or a key, and `pointerdown` for the scrollbar being taken
/// hold of. Nothing here is a scroll on its own — each is the gesture that
/// causes one, and what it says is that the scroll about to arrive is the
/// human's.
export const BY_HAND = ["wheel", "pointerdown", "keydown"];

/// How near the bottom still counts as at it, in pixels.
///
/// A few, because the bottom of a box is a sub-pixel number and a view a
/// fraction short of it is a view at the end of the record. Not more than a
/// few: a line of prose is taller than this, so a human who has scrolled up to
/// read something has unmistakably left the bottom.
const AT_THE_BOTTOM = 8;

/// Where a box stands and how far it goes, asked of whichever of the two boxes
/// this is — an element that scrolls, or the window where nothing does.
function place(box: HTMLElement | null): { at: number; most: number } {
  return box === null
    ? {
        at: window.scrollY,
        most: Math.max(
          document.documentElement.scrollHeight - window.innerHeight,
          0,
        ),
      }
    : {
        at: box.scrollTop,
        most: Math.max(box.scrollHeight - box.clientHeight, 0),
      };
}

/// Whether a box is at its bottom, within the few pixels that still count as
/// there.
function atBottom(box: HTMLElement | null): boolean {
  const { at, most } = place(box);
  return at >= most - AT_THE_BOTTOM;
}

/// Put a box at its bottom.
///
/// Instant rather than animated: this is a view keeping up with a record being
/// written, and a glide would be motion under every line a session says.
function toBottom(box: HTMLElement | null): void {
  const { most } = place(box);

  if (box === null) {
    window.scrollTo(0, most);
  } else {
    box.scrollTop = most;
  }
}

/// Hold the view at the bottom of something that is still growing, until the
/// human takes it off the bottom — and again once they put it back.
///
/// `anchor` is anything drawn inside the box that scrolls: what moves is found
/// from it, so a caller says where its content is rather than which pane it is
/// in. `live` is whether there is anything still to follow — a record that has
/// stopped growing is left exactly where the human left it. `grew` is read in a
/// tracking scope and is what says the content changed: every time it does, the
/// view goes back to the bottom.
///
/// It starts pinned, which is what makes opening a running record land at its
/// end: that is where what is being said now is.
///
/// The pause is the human's own scrolling and nothing else. Content arriving
/// under a pinned view fires no scroll — the box is already at its end and this
/// keeps it there — and this function's own scrolling is not a gesture, so
/// neither can unpin it. Only a scroll that follows one of [`BY_HAND`] is read
/// as the human saying where they want to be.
export function followBottom(
  anchor: Accessor<HTMLElement | undefined>,
  live: Accessor<boolean>,
  grew: Accessor<unknown>,
): void {
  /// Whether the view is being held at the bottom.
  let pinned = true;

  /// Whether the human has touched the scroll at all yet. Until they have,
  /// every scroll there has been is one of this function's own.
  let hand = false;

  // The listeners, for as long as there is something to follow. Set up apart
  // from the following itself so that content arriving does not tear them down
  // and build them again — and take the pin with them when it did.
  createEffect(() => {
    if (!live()) return;

    const from = anchor();
    if (from === undefined) return;

    const box = scroller(from);
    const scrolls: EventTarget = box ?? window;

    const took = () => {
      hand = true;
    };

    const moved = () => {
      if (hand) pinned = atBottom(box);
    };

    for (const gesture of BY_HAND) {
      window.addEventListener(gesture, took);
    }
    scrolls.addEventListener("scroll", moved);

    onCleanup(() => {
      for (const gesture of BY_HAND) {
        window.removeEventListener(gesture, took);
      }
      scrolls.removeEventListener("scroll", moved);
    });
  });

  // And the following: back to the bottom every time the content grows, unless
  // the human is reading somewhere else.
  createEffect(() => {
    grew();

    if (!live()) return;

    const from = anchor();
    if (from === undefined || !pinned) return;

    toBottom(scroller(from));
  });
}
