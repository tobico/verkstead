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
//! **Except for the one card nobody opened.** A join arriving from another
//! machine raises a modal over whatever the human was reading, and there is no
//! way back for a press on the backdrop to be: `insist` is what turns both those
//! ways out off, so the only ways out of that one are the two presses on it.
//! See `Joining.tsx`, which is the whole of why the flag exists.
//!
//! Whether it is open is the caller's, though, rather than this component's: one
//! modal is opened from several places — a button that adds, a row that rewrites
//! — and which of them it was is the caller's to hold anyway. So it is told
//! whether it is up and handed the way to say it has closed itself, which is
//! what Escape and a press on the backdrop come back as.

import { Show, onCleanup, onMount } from "solid-js";
import type { JSX } from "solid-js";

import styles from "./Modal.module.css";

/// What a modal is told, whether it is up or not.
type Sheet = {
  /// Which modal this is, put on the dialog so the caller can size this one's
  /// card. The shared chrome is `Modal.module.css` — the dialog, and the card
  /// underneath it.
  class: string;
  /// Said when the modal has closed itself, which is what Escape and a press on
  /// the backdrop come back as. Nothing here changes `open`; the caller does,
  /// along with whatever else it keeps beside it.
  ///
  /// **Once, and never for a close the caller made.** Taking this modal away is
  /// how a caller answers its own Cancel, and being told about that afterwards
  /// would be being told twice — which costs nothing where a `close` only puts a
  /// signal back, and is a second Repo landing on a draft where it does more.
  close: () => void;
  /// What a screen reader calls the dialog, where the card's own heading is not
  /// what names it.
  name?: string;
  /// The heading inside the card that names it, by id, for the usual case where
  /// there is one.
  labelledBy?: string;
  /// Whether this card insists on being answered: Escape and a press on the
  /// backdrop do nothing, and the only ways out are the caller's own.
  ///
  /// **Off for every modal a press opened**, which is all of them but one: the
  /// human opened it, the way back is the way out, and a card that would not
  /// close is a card that has taken the page hostage.
  ///
  /// **On for the one nobody opened.** A join arrives from another machine
  /// while somebody is reading something else — see `Joining.tsx` — and there
  /// is no way back to close to, because there was nowhere they were going. A
  /// press away from it would leave the question unanswered with nothing on the
  /// page left saying it had been asked, and the human finding out ten minutes
  /// later that the link they were setting up had quietly run out.
  insist?: boolean;

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
        insist={props.insist}
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

  /// Whether the caller has already taken this modal away, which is what makes
  /// the close below its own doing rather than something to tell the caller
  /// about — see `close` on [`Sheet`], which is only ever *this closed itself*.
  let taken = false;

  // Closed on the way out, even where it is the caller taking it away: a modal
  // dialog merely removed from the document leaves the top layer without handing
  // the focus back, and the button that opened this is where the focus belongs.
  //
  // Marked first, because `close()` fires the event below on a node that is on
  // its way out of the document but still carries its listeners: unmarked, a
  // caller whose way out is the thing that unmounts this would be told it had
  // closed a second time, having already acted on the first. Which every caller
  // survived while every `close` did nothing but put a signal back — and the
  // Create repo card's does not: it lands the Repo it made on the draft, and
  // landing it twice is two moves on a saved one.
  onCleanup(() => {
    taken = true;
    if (dialog.open) dialog.close();
  });

  return (
    <dialog
      class={`${styles.modal} ${props.class}`}
      ref={dialog}
      aria-label={props.name}
      aria-labelledby={props.labelledBy}
      // Every way this closes itself comes through here — Escape, and the
      // backdrop below — and nothing else does: a close the caller caused is
      // marked above and says nothing back.
      onClose={() => {
        if (!taken) props.close();
      }}
      // And the one that insists is the one Escape does not reach. `cancel` is
      // what Escape fires and it is cancelable, so refusing it is the platform's
      // own way of saying this dialog has no way out but its contents — rather
      // than the key being swallowed somewhere up the page.
      onCancel={(event) => {
        if (props.insist) event.preventDefault();
      }}
      // The one way out `dialog` has no opinion about. A press on the backdrop
      // lands on the dialog itself, which is why the card underneath carries the
      // padding: with any of its own, a press on the card's margin would read as
      // a press away from it.
      onClick={(event) => {
        if (event.target === dialog && !props.insist) dialog.close();
      }}
    >
      <div class={styles.card}>{props.children}</div>
    </dialog>
  );
}
