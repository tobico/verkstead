//! A run waiting an account's window out, and the one thing the human can do
//! about it.
//!
//! Nothing has gone wrong here, which is the whole difference between this and
//! the interruption it is drawn like: the account is out of window, the agent is
//! waiting for the same reset, and Verkstead has stopped launching anything so
//! that a run which has stopped says so instead of going quiet.
//!
//! So there is one press rather than three remedies, and it is not a decision to
//! be talked into: *go on without waiting* is the only thing the human can add,
//! and it may well be the wrong thing to do — the window is coming back either
//! way. The wait ends by itself when it does.
//!
//! Whole on the timeline, with nothing behind a details pane. What it has to say
//! is a profile's name, a time and the line the session printed, and a pane that
//! had to fetch those could draw the button before it could say what for.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, createSignal, type JSX } from "solid-js";

import { resumePause } from "../api/client";
import type {
  By,
  ConversationView,
  PauseEvent,
  PauseResumed,
} from "../api/types";
import { ErrorLine, Note } from "../notices";
import { utcStamp } from "../set/when";

/// What each way the wait ended is called.
///
/// One record for the line the timeline gives it afterwards, so what the human
/// pressed and what they read back cannot come to be called different things —
/// the same arrangement the remedies have.
export const RESUMED_BY: Record<By, string> = {
  Human: "Went on without waiting",
  Reset: "The window came back",
};

/// And each way of being refused the press.
///
/// `Resumed` is here for completeness of the mapping and never drawn: nothing is
/// said about a press that worked, because the event reading back resumed is what
/// says it.
export const RESUME_REFUSAL: Record<PauseResumed, string> = {
  Resumed: "",
  NoSuchPause: "This pause is gone.",
  AlreadyResumed:
    "The wait was already over — the window came back, or a second press. The first ending stands.",
};

/// A run waiting an account's window out, as the timeline shows it: which
/// account, when it comes back, and the press that says not to wait.
///
/// A card rather than a line, and not a button either — the same shape an
/// interruption has, and for the same reason: there is something to press inside
/// it.
///
/// Once it is over the press goes and what ended it stays, because the record is
/// what a timeline is: a long run against a busy account collects one of these a
/// day, each saying how that day's wait ended.
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
        {(resumed) => (
          <p class="resumed" classList={{ [resumed().by]: true }}>
            {RESUMED_BY[resumed().by]}
          </p>
        )}
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
      <Note>
        The run starts again from where it stopped. Nothing changes about the
        account, and the worktree is left exactly as the session left it — the
        agent is waiting for the same reset, so it may simply wait again.
      </Note>

      <Show when={refused()}>
        {(outcome) => <ErrorLine>{RESUME_REFUSAL[outcome()]}</ErrorLine>}
      </Show>
      <Show when={resume.isError}>
        <ErrorLine>
          The run could not be started again: {resume.error?.message}
        </ErrorLine>
      </Show>
    </div>
  );
}
