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
//! carried, and the submit is the move alone. **Wrapping** needs no payload
//! either — the wrap-up's watchers work out for themselves what is left to do —
//! but it does need a pairing, because sessions run there. The two targets that
//! carry a payload arrive with the tasks that build what each of them starts.
//!
//! **The pairing is the conversation's, not the session's.** It is prefilled
//! from what the conversation already runs the work under and what is picked is
//! recorded as the conversation's own: steering re-settles what runs the work.
//! A steered draft has none fixed yet, which is why the pick is part of the form
//! rather than an error path.
//!
//! **Interrupt current task** is the one thing here that is about the world
//! rather than about the move. A session left alone is seen out to its own end,
//! which is what the click promised; ticking the box ends it where it stands,
//! and the step is left however far it had got. It is drawn only where the click
//! found a session running — there is otherwise nothing to interrupt.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createMemo, createSignal, type JSX } from "solid-js";

import { listProfiles, steer } from "../api/client";
import type {
  ConversationSteered,
  ConversationView,
  SteerTarget,
} from "../api/types";
import { useReading } from "../freshness";
import { Modal } from "../Modal";
import * as pairing from "../pairing";
import { Picker } from "../picking";

/// Each way of being refused a steer.
///
/// Nothing here is about the state the conversation was in: every state is
/// somewhere to steer *from*, so what is left to be wrong about is the target —
/// a state whose work cannot be set going from what the record holds.
export const STEER_REFUSAL: Record<ConversationSteered, string> = {
  Steered: "",
  NoSuchConversation: "This conversation is gone.",
  NoPullRequest:
    "This work is on no pull request, so there is no wrap-up to steer it into.",
  NoPairing: "Pick the account and model the work runs under from here.",
  NoSuchProfile: "That profile has been removed.",
  NoSuchModel: "That profile no longer lists that model.",
  NowhereToWork: "There is no worktree on the record to work in.",
  WorktreeRefused:
    "Its worktree is not one any more, and git would not make it again from the branch.",
};

/// Where a steer can send a conversation, and what each target means.
///
/// Draft and closed are not here and never will be: each has a way in of its
/// own. The other two targets arrive with the tasks that build what each of them
/// launches — a target the modal offers is a target something runs for.
///
/// `runs` is whether work goes on in that state, which is the one question the
/// rest of the form follows from: a target something runs in needs a pairing
/// settled, and one nothing runs in needs none.
const TARGETS: {
  target: SteerTarget;
  label: string;
  note: string;
  runs: boolean;
}[] = [
  {
    target: "Wrapping",
    label: "Wrapping up",
    note: "The branch looked at again: the checks watched, the review run, the comments answered. The fix attempts start over.",
    runs: true,
  },
  {
    target: "Done",
    label: "Done",
    note: "Finished with. Nothing runs, so there is nothing to pick and nothing to write.",
    runs: false,
  },
];

/// Whether this conversation's work is on a pull request.
///
/// What decides whether wrapping up is offered at all. A wrapping conversation
/// is defined by the pull request under it — the record holds the move and the
/// pull request as one act — so a steer there is a move onto one that is already
/// there rather than a way of opening one. Read off the pinned events, which is
/// where the record's own pull request is drawn from.
function onAPullRequest(conversation: ConversationView): boolean {
  return conversation.pinned.some((pinned) => "PullRequest" in pinned);
}

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

  /// The targets this conversation can actually be sent to. Wrapping up is
  /// drawn out where the work is on no pull request: a target that would be
  /// refused by name is worse than one that was never offered.
  const offered = createMemo(() =>
    TARGETS.filter(
      (offered) =>
        offered.target !== "Wrapping" || onAPullRequest(props.conversation),
    ),
  );

  // Where it goes. Prefilled with the first target offered rather than left
  // empty: a picker with nothing picked would be a form the human has to answer
  // twice. The list is never empty — done is offered on every conversation there
  // is — and the fallback is that same target said twice rather than a state
  // this can be in.
  const [target, setTarget] = createSignal<SteerTarget>(
    offered()[0]?.target ?? "Done",
  );

  /// Whether the target picked is one work goes on in, which is what draws the
  /// pairing picker under it.
  const runs = createMemo(
    () => offered().find((one) => one.target === target())?.runs ?? false,
  );

  // The profile list is read here rather than passed in, so the picker is whole
  // wherever the modal is opened from — the setup pane does the same.
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,

    // Merged by the id each row carries flat: a rebuilt `<option>` is a new
    // element in a `<select>` the human may have open, and a list re-read while
    // they were choosing would take the choice with it.
    freshness: { reconcile: "id" },
  }));

  // What the work runs under from here, prefilled from what the conversation
  // already runs it under. The empty string is a conversation with none fixed
  // yet — a steered draft — which is a pick to make rather than an error.
  const [picked, setPicked] = createSignal(
    pairing.chosen(props.conversation.implementation_pairing),
  );

  const [interrupt, setInterrupt] = createSignal(false);
  const [refused, setRefused] = createSignal<ConversationSteered | null>(null);

  const submit = useMutation(() => ({
    mutationFn: () =>
      steer(props.conversation.id, {
        target: target(),
        interrupt: interrupt(),
        // Sent only where the target runs something. A target nothing runs in
        // settles no pairing, and a null there would be the form arguing with
        // itself about what it had picked.
        pairing: runs() && picked() ? pairing.choice(picked()) : null,
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

      // A pairing refused is a profile list this modal read a moment ago, so
      // that is re-read too.
      void queries.invalidateQueries({ queryKey: ["profiles"] });
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
          <For each={offered()}>
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

        {/* Only where something runs in the state picked. What is settled here
            is the conversation's own pairing rather than one session's, which is
            what the line under it says: steering re-settles what runs the work.

            A [`Picker`] rather than a `<select>`, so this cannot come to show
            one pairing while the submit would send another — see
            `src/picking.tsx`. */}
        <Show when={runs()}>
          <div class="steer-pairing">
            <label for="steer-pairing">Run it under</label>
            <Picker
              id="steer-pairing"
              options={pairing.pairings(profiles.data ?? [])}
              value={pairing.value}
              label={pairing.label}
              chosen={picked()}
              pick={setPicked}
              gone={() => setPicked("")}
              disabled={submit.isPending}
            />
            <p class="note">
              What the work runs under from here. This is recorded as the
              conversation's, not just this run's.
            </p>
            <Show when={profiles.isError}>
              <p class="error">
                Could not read the agent profiles: {profiles.error?.message}
              </p>
            </Show>
          </div>
        </Show>

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
          {/* Held shut where the target runs something and nothing is picked to
              run it: the server refuses that by name, and a press that could
              only be refused is one the human should not have to make. */}
          <button
            type="submit"
            class="steer"
            disabled={submit.isPending || (runs() && !picked())}
          >
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
