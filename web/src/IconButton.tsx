//! The icon that is pressed, wherever a pane holds a way into something the
//! pane does not have room to name: the gear at the head of the conversations,
//! and the pluses that add a Profile or a Repo.
//!
//! The same metaphor as [`CardButton`](./CardButton.tsx), which is the whole
//! reason this carries an open state: it is another thing standing in a pane
//! that can be selected and opened into a subpane, so the one that is open is
//! painted as the selected one — in step with how an open card reads, rather
//! than in a second voice of its own.
//!
//! Open is the caller's, never this button's. What is open is a fact about
//! where the human is — the URL, in every case there is — so pressing the open
//! one does nothing new: it makes the same press again, and the same press
//! again changes nothing. There is no toggle here, and a card does not toggle
//! either.
//!
//! An icon says nothing when it is read aloud, so the label is not optional the
//! way [`Icon`](./Icon.tsx)'s is: the button is named by the word handed in
//! here, and the icon inside it is hidden from a screen reader that would
//! otherwise have nothing to say about it.

import type { JSX } from "solid-js";

import type { IconDefinition } from "@fortawesome/free-solid-svg-icons";

import { Icon } from "./Icon";
import styles from "./IconButton.module.css";

/// One pressable icon.
export function IconButton(props: {
  /// The shape, imported by whoever wants it drawn — the definition rather
  /// than a name, so an icon nobody presses is an icon the bundle does not
  /// carry.
  of: IconDefinition;

  /// What the button is called. The icon says none of it, so this is the whole
  /// of what a screen reader has.
  label: string;

  /// Whether this is the button whose pane is open. The one state it draws
  /// about itself, and the whole of what makes it look unlike its neighbours.
  open: boolean;

  /// What pressing it does. Made whatever state the button is in: a press on
  /// the open one is the way it was opened, made again.
  press: () => void;

  /// Whether there is nothing behind it to press into. The caller's, like
  /// `open`, and the caller says why in the `label`: an icon that does nothing
  /// when it is pressed says nothing about why, and the label is the whole of
  /// what a screen reader has of one either way.
  disabled?: boolean;

  /// A class of the caller's, for where it stands in the row it was drawn
  /// into. Styled by whoever passes it, never here.
  class?: string;
}): JSX.Element {
  return (
    <button
      type="button"
      class={[
        styles.iconButton,
        props.open ? styles.open : undefined,
        props.class,
      ]
        .filter(Boolean)
        .join(" ")}
      aria-label={props.label}
      aria-pressed={props.open}
      disabled={props.disabled}
      onClick={() => props.press()}
    >
      <Icon of={props.of} />
    </button>
  );
}
