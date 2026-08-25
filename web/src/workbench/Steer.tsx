//! Steering a conversation: the modal the menu row opens, and what it settles.
//!
//! The click has already happened by the time any of this is drawn — see the
//! menu row in [`Timeline`](./Timeline.tsx), which posts it and opens this on
//! what comes back. That press stopped the drive, so nothing new is launching
//! while the human composes, and **cancel** is no press at all: the conversation
//! stays where the click left it, stopped, with resume drawn on it.
//!
//! What the modal is, therefore, is a form over one question — where does this
//! go? — with whatever that target needs under it. **Done** needs nothing: there
//! is nothing to drive in done, so no pairing is picked and no payload is
//! carried, and the submit is the move alone. The targets that start something
//! arrive with the tasks that build what each of them starts.
//!
//! **Interrupt current task** is the one thing here that is about the world
//! rather than about the move. A session left alone is seen out to its own end,
//! which is what the click promised; ticking the box ends it where it stands,
//! and the step is left however far it had got. It is drawn only where the click
//! found a session running — there is otherwise nothing to interrupt.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createSignal, type JSX } from "solid-js";

import { steer } from "../api/client";
import type {
  ConversationSteered,
  ConversationView,
  SteerTarget,
} from "../api/types";
import { Modal } from "../Modal";

/// Each way of being refused a steer.
///
/// One entry, because the press has one way of failing: the human has looked at
/// the work and said where it goes, so the state it is in is not something to be
/// refused for — every state is a source. What the targets that launch something
/// can be refused for arrives with them.
export const STEER_REFUSAL: Record<ConversationSteered, string> = {
  Steered: "",
  NoSuchConversation: "This conversation is gone.",
};

/// Where a steer can send a conversation, and what each target means.
///
/// Draft and closed are not here and never will be: each has a way in of its
/// own. The other three targets arrive with the tasks that build what each of
/// them launches — a target the modal offers is a target something runs for.
const TARGETS: { target: SteerTarget; label: string; note: string }[] = [
  {
    target: "Done",
    label: "Done",
    note: "Finished with. Nothing runs, so there is nothing to pick and nothing to write.",
  },
];

/// Which of them the picker starts on: the first of the list above.
const PREFILLED: SteerTarget = "Done";

/// The modal, and everything it settles before the move.
export function Steer(props: {
  conversation: ConversationView;
  /// Whether the click found a session still running, which is the only thing
  /// **Interrupt current task** is offered against.
  working: boolean;
  /// Said when the modal has gone, however it went — cancelled, escaped,
  /// pressed away, or submitted.
  close: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  // Where it goes. Prefilled rather than left empty: a picker with nothing
  // picked would be a form the human has to answer twice.
  const [target, setTarget] = createSignal<SteerTarget>(PREFILLED);
  const [interrupt, setInterrupt] = createSignal(false);
  const [refused, setRefused] = createSignal<ConversationSteered | null>(null);

  const submit = useMutation(() => ({
    mutationFn: () =>
      steer(props.conversation.id, {
        target: target(),
        interrupt: interrupt(),
      }),
    onSuccess: (outcome: ConversationSteered) => {
      // The page it was submitted from is out of date either way: the work has
      // moved, or the world had moved under the modal. Reading it again is both
      // the correction and, where it was refused, the explanation.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });

      if (outcome === "Steered") {
        props.close();
        return;
      }

      setRefused(outcome);
    },
  }));

  return (
    <Modal
      class="steer-conversation"
      open
      close={props.close}
      labelledBy="steer-title"
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          submit.mutate();
        }}
      >
        <h3 id="steer-title">Steer this conversation</h3>
        <p class="note">
          The run has stopped while you decide. Cancel leaves it stopped, with
          resume on offer.
        </p>

        {/* One block per target, each saying what it means, for the reason the
            actions menu gives each of its presses a line: the words between two
            of these are the difference between an hour of work and none. */}
        <fieldset class="steer-targets">
          <legend>Move it into</legend>
          <For each={TARGETS}>
            {(offered) => (
              <div class="steer-target">
                <label>
                  <input
                    type="radio"
                    name="steer-target"
                    value={offered.target}
                    checked={target() === offered.target}
                    onChange={() => setTarget(offered.target)}
                  />
                  {offered.label}
                </label>
                <p class="note">{offered.note}</p>
              </div>
            )}
          </For>
        </fieldset>

        {/* Only where there is one to interrupt. With nothing running the box
            would promise something about a session that is not there. */}
        <Show when={props.working}>
          <div class="steer-interrupt">
            <label>
              <input
                type="checkbox"
                checked={interrupt()}
                onChange={(event) => setInterrupt(event.currentTarget.checked)}
              />
              Interrupt current task
            </label>
            <p class="note">
              End the session running now where it stands. Left alone it finishes
              what it was doing, and nothing is started after it.
            </p>
          </div>
        </Show>

        <div class="steer-buttons">
          <button type="submit" class="steer" disabled={submit.isPending}>
            {submit.isPending ? "Steering…" : "Steer"}
          </button>
          {/* Drawn as well as the ways out the modal already has: escape and a
              press on the backdrop are for a keyboard and a cursor, and this is
              the one a thumb has. */}
          <button type="button" class="cancel" onClick={props.close}>
            Cancel
          </button>
        </div>

        <Show when={refused()}>
          {(outcome) => <p class="error">{STEER_REFUSAL[outcome()]}</p>}
        </Show>
        {/* A server that could not answer at all, which is the one thing here
            that is an error rather than an outcome. */}
        <Show when={submit.isError}>
          <p class="error">
            The conversation could not be steered: {submit.error?.message}
          </p>
        </Show>
      </form>
    </Modal>
  );
}
