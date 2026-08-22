//! A run that stopped: what went wrong, and the three things the human can do
//! about it.
//!
//! Roadrunner asked this over askance, because nobody was at its terminal. Here
//! the timeline *is* where the human looks, so the same question is GUI-native:
//! the event carries the remedies, and the conversation carries *blocked on you*
//! while it waits.
//!
//! Both halves of the event live in this file. The timeline draws the summary
//! with the remedies on it — the design puts an interruption inline *with remedy
//! actions*, because a run that has stopped is a thing to answer rather than a
//! thing to go and open. The details pane draws the evidence: what git made of
//! the worktree, and the tail of what the session last said.
//!
//! The evidence rides on the event rather than being fetched, unlike a
//! Capture or a diff. It was bounded when it was gathered, and it is what the
//! remedies are chosen against — a pane that had to fetch it could draw the
//! buttons before it could say what they were for.
//!
//! Nothing here reverts anything, and the wording says so: in every case the repo
//! is left exactly as the session left it. That is what makes *take over
//! manually* a remedy at all.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createSignal, type JSX } from "solid-js";

import { settleInterruption } from "../api/client";
import type {
  ConversationView,
  InterruptionEvent,
  Remedy,
  RemedySettled,
} from "../api/types";

/// What each remedy is called, and what choosing it does.
///
/// One record for the button and for the line the timeline gives the choice
/// afterwards, so what the human pressed and what they read back cannot come to
/// be called different things — the same arrangement the direction chooser has.
export const REMEDY: Record<Remedy, string> = {
  Retry: "Retry",
  TakeOver: "Take over manually",
  Abort: "Abort the run",
};

/// The three, in the order the design names them, each with what it means.
const REMEDIES: { remedy: Remedy; note: string }[] = [
  {
    remedy: "Retry",
    note: "Runs this step again in a fresh session, told whatever you write below.",
  },
  {
    remedy: "TakeOver",
    note: "Verkstead stops driving, so you can take this step on yourself. The worktree is left where it is.",
  },
  {
    remedy: "Abort",
    note: "The run ends here. Nothing else is started.",
  },
];

/// And each way of being refused one.
///
/// `Settled` is here for completeness of the mapping and never drawn: nothing is
/// said about a remedy that worked, because the event reading back settled is
/// what says it.
export const REMEDY_REFUSAL: Record<RemedySettled, string> = {
  Settled: "",
  NoSuchInterruption: "This interruption is gone.",
  AlreadySettled:
    "This was already answered — from another device, or by a second press. The first choice stands.",
};

/// A run that stopped, as the timeline shows it: which step failed, how it
/// ended, and what to do about it.
///
/// A card rather than a line, and not a button either. Every other event with a
/// full self is a button that opens the details pane; this one has something to
/// press *inside* it, so the way into the pane is a control of its own rather
/// than the whole card.
///
/// Once it is settled the remedies go and the choice stays, because the record is
/// what a timeline is: a run that was retried and stopped again has both stops on
/// it, each saying what was decided.
export function Interruption(props: {
  conversation: ConversationView;
  stopped: InterruptionEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const open = () => props.stopped.settled === null;

  return (
    <article
      class="interruption"
      classList={{ selected: props.selected, open: open() }}
    >
      <div class="event-head">
        <h2>Interruption</h2>
        <Show when={open()}>
          <span class="live">blocked on you</span>
        </Show>
        <button type="button" class="open-event" onClick={props.open}>
          Evidence
        </button>
      </div>

      <p class="what">{props.stopped.what}</p>
      <p class="how">{props.stopped.how}</p>

      <Show
        when={props.stopped.settled}
        fallback={
          <Remedies
            conversation={props.conversation}
            stopped={props.stopped}
          />
        }
      >
        {(settled) => (
          <p class="settled" classList={{ [settled().remedy]: true }}>
            {REMEDY[settled().remedy]}
            <Show when={settled().note !== ""}>
              <span class="note">{settled().note}</span>
            </Show>
          </p>
        )}
      </Show>
    </article>
  );
}

/// The three remedies, and what the human wants said alongside.
///
/// The note is one field for all three rather than one per button, because it is
/// the same thing in each case: what the human wants on the record about this
/// decision. Only a retry passes it on to an agent, which is what its own label
/// says.
///
/// Nothing is preselected and nothing is recommended. Verkstead noticed the run
/// stop and cannot resolve it — that is what an interruption *is* — so it has no
/// opinion about which of the three is right.
function Remedies(props: {
  conversation: ConversationView;
  stopped: InterruptionEvent;
}): JSX.Element {
  const queries = useQueryClient();

  const [note, setNote] = createSignal("");
  const [refused, setRefused] = createSignal<RemedySettled | null>(null);

  const settle = useMutation(() => ({
    mutationFn: (remedy: Remedy) =>
      settleInterruption(
        props.conversation.id,
        props.stopped.id,
        remedy,
        note(),
      ),
    onSuccess: (outcome: RemedySettled) => {
      if (outcome !== "Settled") {
        setRefused(outcome);
      } else {
        setRefused(null);
      }

      // Either way: settled is a timeline that has moved, and refused is a
      // picture of the world this page read a moment ago — reading it again is
      // both the correction and the explanation.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <div class="remedies">
      <label for={`note-${props.stopped.id}`}>
        Anything to say about it
      </label>
      {/* A copy of what has been typed gives the field its height — see
          `.grow`, which the brief's field uses for the same reason. */}
      <div class="grow" data-value={note()}>
        <textarea
          id={`note-${props.stopped.id}`}
          rows="1"
          placeholder="Try again, but leave the migration alone"
          value={note()}
          onInput={(ev) => {
            setNote(ev.currentTarget.value);
            setRefused(null);
          }}
        />
      </div>

      <ul class="remedy-list">
        <For each={REMEDIES}>
          {(offered) => (
            <li classList={{ [offered.remedy]: true }}>
              <button
                type="button"
                class="remedy"
                disabled={settle.isPending}
                onClick={() => settle.mutate(offered.remedy)}
              >
                {REMEDY[offered.remedy]}
              </button>
              <p class="note">{offered.note}</p>
            </li>
          )}
        </For>
      </ul>

      <p class="note left-as-it-is">
        Whichever you pick, the repo is left exactly as the session left it.
      </p>

      <Show when={refused()}>
        {(outcome) => <p class="error">{REMEDY_REFUSAL[outcome()]}</p>}
      </Show>
      <Show when={settle.isError}>
        <p class="error">
          The interruption could not be settled: {settle.error?.message}
        </p>
      </Show>
    </div>
  );
}

/// The evidence, opened: what git made of the worktree, and the tail of what the
/// session last said.
///
/// Both were read at the moment the run stopped and kept, because both move on —
/// a worktree is a directory the human also has, and a session's output belongs
/// to a process that has gone. So this is a reading of how things were, not of
/// how they are, and it says so.
///
/// Preformatted and not rendered. Neither `git status` nor a terminal's last
/// words are markdown, and the columns are the whole of what makes a status
/// readable.
export function Evidence(props: {
  stopped: InterruptionEvent;
  back: () => void;
  close: () => void;
}): JSX.Element {
  return (
    <>
      <div class="pane-head">
        <button type="button" class="pane-back" onClick={props.back}>
          ← Timeline
        </button>
        <h1>Interruption</h1>
        <button type="button" class="close-event" onClick={props.close}>
          Close
        </button>
      </div>

      <div class="interruption-summary">
        <p class="what">{props.stopped.what}</p>
        <p class="how">{props.stopped.how}</p>
      </div>

      <section class="evidence">
        <div class="section-head">
          <h2 class="section-heading">Worktree</h2>
        </div>
        <Show
          when={props.stopped.git_status !== ""}
          fallback={
            <p class="empty">
              Git had nothing pending, or the repo would not answer.
            </p>
          }
        >
          <pre class="git-status">{props.stopped.git_status}</pre>
        </Show>
      </section>

      <section class="evidence">
        <div class="section-head">
          <h2 class="section-heading">What the session last said</h2>
        </div>
        <Show
          when={props.stopped.tail !== ""}
          fallback={<p class="empty">It printed nothing at all.</p>}
        >
          <pre class="tail">{props.stopped.tail}</pre>
        </Show>
        <p class="note">
          The tail of it, as it stood when the run stopped. The whole capture
          is the session's own event, further up the timeline.
        </p>
      </section>
    </>
  );
}
