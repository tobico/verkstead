//! The press a hand that was not holding perfectly still makes, staged for
//! jsdom — see `rowPress` in `src/rows.ts`, which is what answers it.
//!
//! Written here rather than at each of the places a row is pressed, because it
//! is one gesture and the reason it has to be staged at all takes a paragraph.
//!
//! **The browser's part in a slid press is a withholding.** A `<label>`
//! activates its control by forwarding its click to it, and the browser forwards
//! nothing the moment the pointer moved at all between the press and the
//! release. Driven in Firefox, four pixels is enough. So the row is handed its
//! own click and the control is handed none — and a row that answered only the
//! forwarded one did nothing, leaving a character of the words highlighted where
//! an answer should have been.
//!
//! jsdom has no pointer, no selection and no drag, and it forwards a label's
//! click unconditionally. There is nothing there to slide, so what is staged is
//! the withholding: the forwarding is cancelled from a capture listener above
//! the page, which is the one thing about a slid press a row can tell. What the
//! row is handed is then the same either way — its own click, and none on the
//! control — which is the whole of what these tests ask about.
//!
//! A still press needs no staging: `fireEvent.click` on the row is one, and
//! jsdom forwards it as a browser would.

import { fireEvent } from "@solidjs/testing-library";

/// A press that landed on `on` and slid before it was let go.
///
/// `on` is where the press landed rather than the row that answers it: the words
/// of an Option, the name of a direction, a cell of an Answer Table. The click
/// bubbles to the row from there, exactly as the browser's would.
export function slidPress(on: Element): void {
  const withhold = (event: Event) => event.preventDefault();

  document.addEventListener("click", withhold, true);
  try {
    fireEvent.click(on);
  } finally {
    document.removeEventListener("click", withhold, true);
  }
}
