//! The one modal in the UI: a card drawn over the page, with the page held
//! behind it until it is answered.
//!
//! Native `dialog`, opened with `showModal`, because everything a modal owes the
//! human is already in the platform and none of it is worth writing again: the
//! top layer, so nothing on the page can be drawn over it; the backdrop, and the
//! page behind it going inert; Escape; and the focus moved in on the way open
//! and handed back to whatever opened it on the way shut. What is left to write
//! is the one thing `dialog` has no opinion about — that a press away from the
//! card takes it back — and the paint.
//!
//! Which paint is the confirm sheet's, because that is what this UI already
//! called a thing drawn over the page: dimmed behind, rising from the bottom
//! edge where a thumb is, and centred once the window is wider than a phone.
//!
//! The contents stay with the caller, as a menu's rows do. No two of these hold
//! the same kind of thing — a form of four fields, one irreversible question —
//! and a component that tried to own a title and a row of buttons would be
//! guessing at both. What it owns is where the card sits and every way out of
//! it.
//!
//! Whether it is open is the caller's, though, rather than this component's: one
//! modal is opened from several places — a button that adds, a row that rewrites
//! — and which of them it was is the caller's to hold anyway. So it is told
//! whether it is up and handed the way to say it has closed itself, which is
//! what Escape and a press on the backdrop come back as.

import { Show, onCleanup, onMount } from "solid-js";
import type { JSX } from "solid-js";

/// What a modal is told, whether it is up or not.
type Sheet = {
  /// Which modal this is, put on the dialog so the stylesheet can size this
  /// one's card. The shared chrome is `.modal` and `.modal-card` underneath it.
  class: string;
  /// Said when the modal has closed itself, which is what Escape and a press on
  /// the backdrop come back as. Nothing here changes `open`; the caller does,
  /// along with whatever else it keeps beside it.
  close: () => void;
  /// What a screen reader calls the dialog, where the card's own heading is not
  /// what names it.
  name?: string;
  /// The heading inside the card that names it, by id, for the usual case where
  /// there is one.
  labelledBy?: string;
  /// The card's contents, whole: whatever the caller would have drawn inline.
  children: JSX.Element;
};

/// A modal, and the card it draws over the page.
export function Modal(
  props: Sheet & {
    /// Whether it is up. Nothing of it is on the page while this is false: a
    /// closed modal is not a hidden one, and its contents are built afresh each
    /// time it opens — which is what a field filled in from the row it was
    /// opened beside depends on.
    open: boolean;
  },
): JSX.Element {
  return (
    <Show when={props.open}>
      <Drawn
        class={props.class}
        close={props.close}
        name={props.name}
        labelledBy={props.labelledBy}
      >
        {props.children}
      </Drawn>
    </Show>
  );
}

/// The dialog itself, which exists only while the modal is up.
///
/// Its own component so that opening it is something that happens on the way
/// into the document: `showModal` refuses a dialog that is not in one yet, and a
/// ref is handed over before it is.
function Drawn(props: Sheet): JSX.Element {
  let dialog!: HTMLDialogElement;

  onMount(() => dialog.showModal());

  // Closed on the way out, even where it is the caller taking it away: a modal
  // dialog merely removed from the document leaves the top layer without handing
  // the focus back, and the button that opened this is where the focus belongs.
  onCleanup(() => {
    if (dialog.open) dialog.close();
  });

  return (
    <dialog
      class={`modal ${props.class}`}
      ref={dialog}
      aria-label={props.name}
      aria-labelledby={props.labelledBy}
      onClose={() => props.close()}
      // The one way out `dialog` has no opinion about. A press on the backdrop
      // lands on the dialog itself, which is why the card underneath carries the
      // padding: with any of its own, a press on the card's margin would read as
      // a press away from it.
      onClick={(event) => {
        if (event.target === dialog) dialog.close();
      }}
    >
      <div class="modal-card">{props.children}</div>
    </dialog>
  );
}
