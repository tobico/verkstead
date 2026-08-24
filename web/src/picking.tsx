//! Picking one of the server's rows out of a dropdown: the one way a `<select>`
//! is built here, and the divergence it does not let happen.
//!
//! What a `<select>` shows is a property of the element and not of its markup,
//! and the browser keeps it plausible on its own: take away the option that was
//! selected and the visible selection snaps to the first one left. Nothing is
//! said about it. The signal behind the control still holds the old choice, and
//! Solid never re-applies `value`, because the string it was given did not
//! change — so the human reads one repository off the screen and the form sends
//! another.
//!
//! The merge of ADR-0009 keeps the option elements alive whenever the list came
//! back the same, which is nearly every re-read. "Nearly" is where a bug class
//! lives, and a wrong repository is not a class to leave one in: the divergence
//! is closed here instead, for every re-read and not just the quiet ones.
//!
//! Two things do it, and they are why this is a component rather than a note in
//! a review. The displayed value is re-applied after the options are rebuilt,
//! every time they are rebuilt, so a browser's fixing-up is undone before
//! anybody sees it. And a choice that is no longer among the options is not
//! shown as one: the control falls to its placeholder, which is the honest
//! reading of *what you picked is gone*, and says so to whoever owns the choice.
//!
//! What is shown and what would be sent are therefore the same string always,
//! which is the whole point of the file.

import { For, Show, createEffect, type JSX } from "solid-js";

/// One of the server's rows, in a dropdown.
///
/// The options are the caller's rows and the two functions read them: `value` is
/// what a pick puts on the wire, `label` is what the human reads. Both because
/// nothing here knows what a row is — the repos, the profiles and whatever comes
/// next carry different fields and the same problem.
export function Picker<T>(props: {
  /// The control's own id, for the `<label>` that names it.
  id: string;
  /// Every row that can be picked, as the last read of the list had them.
  options: T[];
  /// What one row sends when it is the choice.
  value: (option: T) => string;
  /// What one row reads as.
  label: (option: T) => string;
  /// What is chosen now, as `value` would have written it — the empty string
  /// for nothing chosen.
  chosen: string;
  /// What the human just picked. Never the empty string, unless the caller
  /// offered one: the placeholder is a state to be in and not a choice to make,
  /// but an option of the caller's own that sends nothing is a choice like any
  /// other — the base dropdown's first entry, which is the rule to go back to.
  pick: (value: string) => void;
  /// Said when what was chosen is no longer among the options, for the caller
  /// that holds the choice in a signal of its own to clear it.
  ///
  /// Optional because not every choice is the page's to reset: the profile
  /// pickers show what the *server* says the conversation chose, and a viewer
  /// that unpicked it on its own would be arguing with the record. Falling to
  /// the placeholder is the whole of the correction there, and the conversation
  /// read back afterwards is what settles it.
  gone?: () => void;
  disabled?: boolean;
}): JSX.Element {
  /// What each option would send, in the order they are drawn.
  const offered = (): string[] => props.options.map(props.value);

  /// Whether the caller's own list holds an option that sends nothing.
  ///
  /// Where it does, the empty string is one of the choices rather than the
  /// absence of one: no placeholder is drawn over it, and picking it is a pick
  /// like any other. Read off the options rather than taken as a flag, because
  /// it is the same fact either way and two ways of saying it could disagree.
  const offersNothing = (): boolean => offered().includes("");

  /// Whether what is chosen is still something that can be picked.
  const standing = (): boolean => offered().includes(props.chosen);

  /// What the control shows: the choice for as long as it exists, and the
  /// placeholder the moment it does not.
  const shown = (): string => (standing() ? props.chosen : "");

  // The element is built first and the effect made after it, because the order
  // they are created in is the order they run in. The options are rebuilt by a
  // computation this expression makes, so an effect made before it would be
  // re-applying the value to the option set that is on its way out.
  const select = (
    <select
      id={props.id}
      disabled={props.disabled}
      onChange={(ev) => {
        const picked = ev.currentTarget.value;
        if (picked || offersNothing()) {
          props.pick(picked);
        }
      }}
    >
      {/* Drawn only while it is what the control is showing, so a settled
          choice is not sitting next to an invitation to unmake it — there is
          no unpicking here, on any of the pickers.

          The same words on all of them, because it is the same state on all of
          them: a choice has not been made, or the one that was made is gone.
          A picker whose reader would need to be told something else can take
          the words as a prop then.

          Nothing at all where the caller offers an option of its own that sends
          nothing: that option is what the empty string means there, and a
          placeholder over it would be two rows for one state. */}
      <Show when={shown() === "" && !offersNothing()}>
        <option value="">Not chosen</option>
      </Show>
      <For each={props.options}>
        {(option) => (
          <option value={props.value(option)}>{props.label(option)}</option>
        )}
      </For>
    </select>
  ) as HTMLSelectElement;

  // The re-apply. Reading the values is what makes this run again when the
  // option set moves, which is exactly when the browser will have chosen for
  // itself.
  createEffect(() => {
    offered();
    select.value = shown();
  });

  // And the telling. Said as its own effect so the re-apply above does not
  // depend on anybody acting on it: the control is honest either way, and this
  // is the caller's chance to be.
  createEffect(() => {
    if (props.chosen !== "" && !standing()) {
      props.gone?.();
    }
  });

  return select;
}
