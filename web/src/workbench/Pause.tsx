//! A Pause a Verkstead of before put on a timeline, and the one thing the human
//! can do about it.
//!
//! Nothing writes another. An account out of window stops a run the way
//! everything else does now — one notice, one badge, one Resume — and what is
//! drawn here is a record of a wait that happened, kept because ADR-0006's rule
//! is that the record is read rather than rewritten.
//!
//! So there is one press and no remedies: *go on without waiting* takes the
//! stop the wait was read onto its conversation as away, and the run picks up
//! from where it stopped. No wait ends by itself — no stop resumes itself — so
//! a stored one that is still open is waiting on that press and nothing else.
//!
//! Whole on the timeline, with nothing behind a details pane. What it has to say
//! is a profile's name, a time and the line the session printed, and a pane that
//! had to fetch those could draw the button before it could say what for.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, createSignal, type JSX } from "solid-js";

import { resumePause } from "../api/client";
import type { ConversationView, PauseEvent, PauseResumed } from "../api/types";
import { utcStamp } from "../set/when";

/// What the line reads once the wait is over.
///
/// One string, because there is one way a wait ends: the human presses. There
/// was a time when the reset time passing ended one too, and no stop resumes
/// itself now.
export const WENT_ON = "Went on without waiting";

/// And each way of being refused the press.
///
/// `Resumed` is here for completeness of the mapping and never drawn: nothing is
/// said about a press that worked, because the event reading back resumed is what
/// says it.
export const RESUME_REFUSAL: Record<PauseResumed, string> = {
  Resumed: "",
  NoSuchPause: "This pause is gone.",
  AlreadyResumed: "The wait was already over — a second press. The first ending stands.",
};

/// A run waiting an account's window out, as the timeline shows it: which
/// account, when it comes back, and the press that says not to wait.
///
/// A card rather than a line, and not a button either — the same shape an
/// interruption has, and for the same reason: there is something to press inside
/// it.
///
/// Once it is over the press goes and the line saying so stays, because the
/// record is what a timeline is: a long run against a busy account collects one
/// of these a day, each saying that day's wait was ended.
export function Pause(props: {
  conversation: ConversationView;
  waiting: PauseEvent;
  selected: boolean;
}): JSX.Element {
  const open = () => props.waiting.resumed === null;

  return (
    <article
      class="pause"
      classList={{ selected: props.selected, open: open() }}
    >
      <div class="event-head">
        <h2>Paused</h2>
        <Show when={open()}>
          <span class="live">blocked on you</span>
        </Show>
      </div>

      <p class="what">
        {props.waiting.profile} is out of window
        <Show when={props.waiting.resets_at}>
          {(resets) => <> until {utcStamp(resets())}</>}
        </Show>
      </p>
      <p class="how">{props.waiting.said}</p>

      <Show
        when={props.waiting.resumed}
        fallback={
          <Waiting
            conversation={props.conversation}
            waiting={props.waiting}
          />
        }
      >
        <p class="resumed">{WENT_ON}</p>
      </Show>
    </article>
  );
}

/// The press, and what it means.
///
/// Nothing else is offered. Verkstead is not driving anything here and has
/// nothing to retry — the step is where the session left it, and the worktree is
/// untouched — so the only choice there is is whether to keep waiting.
function Waiting(props: {
  conversation: ConversationView;
  waiting: PauseEvent;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<PauseResumed | null>(null);

  const resume = useMutation(() => ({
    mutationFn: () => resumePause(props.conversation.id, props.waiting.id),
    onSuccess: (outcome: PauseResumed) => {
      setRefused(outcome === "Resumed" ? null : outcome);

      // Either way: resumed is a timeline that has moved, and refused is a
      // picture of the world this page read a moment ago — reading it again is
      // both the correction and the explanation.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <div class="resuming">
      <button
        type="button"
        class="resume"
        disabled={resume.isPending}
        onClick={() => resume.mutate()}
      >
        Go on without waiting
      </button>
      <p class="note">
        The run starts again from where it stopped. Nothing changes about the
        account, and the worktree is left exactly as the session left it — so a
        window that has not come back will stop it again.
      </p>

      <Show when={refused()}>
        {(outcome) => <p class="error">{RESUME_REFUSAL[outcome()]}</p>}
      </Show>
      <Show when={resume.isError}>
        <p class="error">The run could not be started again: {resume.error?.message}</p>
      </Show>
    </div>
  );
}
