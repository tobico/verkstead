//! How a field that saves itself does it — the pause before it keeps what is in
//! it, and what happens to a keystroke that lands while a save is in the air.
//!
//! One of each, shared by every self-saving field on a drafting Conversation —
//! the Brief and the branch name — because they are the same pane, and a human
//! typing across both should meet neither two ideas of what a pause is nor two
//! ideas of when what they typed goes out. Its own module because the composer
//! draws the setup the branch stands in, so neither of the two files can own it
//! without the other importing it back.

import { onCleanup } from "solid-js";

/// Long enough that a sentence is one save rather than a save a word, and short
/// enough that a human who typed and then sat back has a saved draft by the
/// time they have read it over. Leaving the field saves it whatever the timer
/// was about to do.
export const SETTLE = 800;

/// What a field that keeps itself has to say about itself, which is three
/// things: whether there is anything to save, whether asking again could ever
/// get a different answer, and how to send what is in it.
interface Field {
  /// Whether the field is ahead of the record, which is the whole of what there
  /// is to save.
  unsaved: () => boolean;

  /// Whether an answer has come back that asking again could not better — the
  /// Brief frozen, the Conversation gone. Nothing more goes out after one.
  settled: () => boolean;

  /// Send what is in the field. Called only when there is something to send and
  /// nothing already in the air.
  save: () => void;
}

/// What the field drives it with: the keystroke, the way out of the field, and
/// the save coming back.
interface Keeping {
  /// A keystroke: start the pause again, and save when it comes round.
  settle: () => void;

  /// Save now — leaving the field, or Enter in it, or the pause coming round.
  keep: () => void;

  /// One save is over, whatever became of it. Call this from the mutation's
  /// `onSettled`.
  done: () => void;
}

/// Keep a field that saves itself: the pause, the one save at a time, and the
/// save of whatever was typed while the last one was in the air.
///
/// One of these for the Brief and for the branch name both, because they are
/// the same card and the same three rules — and two copies of the same three
/// rules is what let them disagree about one of them.
export function keeping(field: Field): Keeping {
  /// Whether a save is in the air.
  ///
  /// Held here rather than read off the mutation, because the callback that
  /// decides what to do next runs before the mutation has been told the save is
  /// over — and what that callback is deciding is whether to start another one.
  let saving = false;

  // The pause: one timer, restarted by every keystroke and cancelled by
  // whatever saves before it comes round.
  let pause: ReturnType<typeof setTimeout> | undefined;

  /// Save what is in the field, if the record does not have it already.
  ///
  /// One save at a time: another started while one is in flight could land in
  /// either order, and the loser would be the record. What was typed meanwhile
  /// goes out from `done` below, the moment the one in flight is over.
  ///
  /// Cancelling the pause here costs nothing that is not sent in the same
  /// breath: where there is anything to save this saves it, and where there is
  /// not, the timer would have come round to the same nothing.
  const keep = () => {
    clearTimeout(pause);
    if (field.settled() || !field.unsaved() || saving) return;

    saving = true;
    field.save();
  };

  onCleanup(() => clearTimeout(pause));

  return {
    settle: () => {
      clearTimeout(pause);
      pause = setTimeout(keep, SETTLE);
    },
    keep,
    done: () => {
      saving = false;
      keep();
    },
  };
}
