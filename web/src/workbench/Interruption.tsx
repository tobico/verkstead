//! A run that stopped: what went wrong, and the three things the human can do
//! about it.
//!
//! Roadrunner asked this over askance, because nobody was at its terminal. Here
//! the timeline *is* where the human looks, so the same question is GUI-native
//! — and it is put the way every other question in the workbench is put: the
//! event is a card to open, and what it opens is a sheet to answer.
//!
//! Both halves of the event live in this file. The timeline draws the summary as
//! a plain openable button, with *blocked on you* on it while nothing has been
//! decided and the remedy chosen on it once something has. The details pane
//! draws the sheet: the evidence — what git made of the worktree, and the tail
//! of what the session last said — above the three remedies, read the way a
//! Set's preface and diff are read before answering it.
//!
//! Nothing acts on a stray tap. The remedies are option rows with one submit
//! under them rather than three buttons, because a remedy is a decision and
//! aborting a run is a decision that cannot be taken back — and because that is
//! exactly how answering already works everywhere else here.
//!
//! The evidence rides on the event rather than being fetched, unlike a
//! Capture or a diff. It was bounded when it was gathered, and it is what the
//! remedies are chosen against — a pane that had to fetch it could draw the
//! sheet before it could say what it was about.
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
/// One record for the option row and for the line the timeline gives the choice
/// afterwards, so what the human picked and what they read back cannot come to
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
/// ended, and where the decision about it stands.
///
/// A button, as a question set and a session's output are, and for the same
/// reason: the whole of it — the evidence and the three remedies — is in the
/// details pane, and this is how it is opened. There is nothing to press inside
/// it any more, which is what let it become one card with one meaning rather
/// than a card with a control in it and a separate way in to the rest.
///
/// Once it is settled the badge goes and the choice stays, because the record is
/// what a timeline is: a run that was retried and stopped again has both stops on
/// it, each saying what was decided.
export function Interruption(props: {
  stopped: InterruptionEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const open = () => props.stopped.settled === null;

  return (
    <button
      type="button"
      class="interruption"
      classList={{ selected: props.selected, open: open() }}
      aria-pressed={props.selected}
      onClick={props.open}
    >
      {/* Spans rather than the blocks this reads as, laid out as blocks by the
          stylesheet: everything here is inside a button, and a button holds
          phrasing. */}
      <span class="event-head">
        <span class="event-kind">Interruption</span>
        <Show when={open()}>
          <span class="live">blocked on you</span>
        </Show>
      </span>

      <span class="what">{props.stopped.what}</span>
      <span class="how">{props.stopped.how}</span>

      {/* What was decided, once it has been — the name of it and nothing else.
          What was written alongside is on the sheet in the pane, where the rest
          of the record is. */}
      <Show when={props.stopped.settled}>
        {(settled) => (
          <span class="settled" classList={{ [settled().remedy]: true }}>
            {REMEDY[settled().remedy]}
          </span>
        )}
      </Show>
    </button>
  );
}

/// An interruption opened: the evidence, and the sheet the remedy is chosen on.
///
/// The evidence comes first because it is what the choice is made against — the
/// place a Set's Preface and Diff sit, and read for the same reason. Under it
/// the three remedies, and one submit.
///
/// The evidence itself is a reading of how things were rather than of how they
/// are, and it says so: both halves were read at the moment the run stopped and
/// kept, because both move on — a worktree is a directory the human also has,
/// and a session's output belongs to a process that has gone.
///
/// Preformatted and not rendered. Neither `git status` nor a terminal's last
/// words are markdown, and the columns are the whole of what makes a status
/// readable.
export function Interrupted(props: {
  conversation: ConversationView;
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
          <Chosen remedy={settled().remedy} note={settled().note} />
        )}
      </Show>
    </>
  );
}

/// The three remedies as a sheet to fill in: pick one, say whatever goes with
/// it, and submit.
///
/// Option rows and one submit, exactly as a Question's Options are answered and
/// a direction is picked. Three buttons that each acted the moment they were
/// pressed were a stray tap away from ending a run — and this pane is walked
/// into on a phone, from a badge, by somebody who came to read what happened.
///
/// The note is one field for all three rather than one per row, because it is
/// the same thing in each case: what the human wants on the record about this
/// decision. Only a retry passes it on to an agent, which is what its own row
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

  const [picked, setPicked] = createSignal<Remedy | null>(null);
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

  /// Send whichever was picked. Nothing is sent with nothing picked — the
  /// submit is disabled there, and this is the same rule said where it is
  /// enforced.
  const send = () => {
    const remedy = picked();
    if (remedy !== null) {
      settle.mutate(remedy);
    }
  };

  return (
    <section class="remedy-pick">
      <h2 class="section-heading">What to do about it</h2>
      <ul class="remedies">
        <For each={REMEDIES}>
          {(offered) => (
            <li class="remedy" classList={{ [offered.remedy]: true }}>
              <label>
                <input
                  type="radio"
                  name={`remedy-${props.stopped.id}`}
                  value={offered.remedy}
                  checked={picked() === offered.remedy}
                  // Both gestures, for the reason an Option answers both: an
                  // arrow key fires a change and never a click, and a click on
                  // what is already picked fires a click and never a change.
                  onChange={() => setPicked(offered.remedy)}
                  onClick={() =>
                    setPicked((held) =>
                      held === offered.remedy ? null : offered.remedy,
                    )
                  }
                />
                <span class="remedy-name">{REMEDY[offered.remedy]}</span>
              </label>
              <p class="note">{offered.note}</p>
            </li>
          )}
        </For>
      </ul>

      {/* The one thing true of all three, said once under them rather than three
          times over — and before the submit, because it is part of what is
          being decided. */}
      <p class="note left-as-it-is">
        Whichever you pick, the repo is left exactly as the session left it.
      </p>

      <section class="remedy-note">
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
      </section>

      <section class="submit">
        <button
          type="button"
          onClick={send}
          disabled={picked() === null || settle.isPending}
        >
          {settle.isPending ? "Sending…" : "Submit"}
        </button>
        <Show when={refused()}>
          {(outcome) => <p class="error">{REMEDY_REFUSAL[outcome()]}</p>}
        </Show>
        <Show when={settle.isError}>
          <p class="error">
            The interruption could not be settled: {settle.error?.message}
          </p>
        </Show>
      </section>
    </section>
  );
}

/// The same sheet, read rather than filled in: the three that were offered, the
/// one that was chosen, and whatever was written alongside it.
///
/// All three stay on the record, as the direction chooser's do — what was turned
/// down is half of the decision, and a run that was aborted is read against the
/// two things that could have been done instead.
function Chosen(props: { remedy: Remedy; note: string }): JSX.Element {
  const said = () => props.note.trim();

  return (
    <section class="remedy-pick decided">
      <h2 class="section-heading">What was done about it</h2>
      <ul class="remedies">
        <For each={REMEDIES}>
          {(offered) => (
            <li
              class="remedy"
              classList={{
                [offered.remedy]: true,
                chosen: props.remedy === offered.remedy,
              }}
            >
              <span class="remedy-name">{REMEDY[offered.remedy]}</span>
              <Show when={props.remedy === offered.remedy}>
                <span class="chose">chosen</span>
              </Show>
              <p class="note">{offered.note}</p>
            </li>
          )}
        </For>
      </ul>
      <p class="note left-as-it-is">
        The repo was left exactly as the session left it.
      </p>
      {/* Only where there is one, exactly as the sheet only ever sends a note
          that has something in it. */}
      <Show when={said() !== ""}>
        <section class="remedy-note decided">
          <h2>Said alongside it</h2>
          <p class="note-said">{said()}</p>
        </section>
      </Show>
    </section>
  );
}
