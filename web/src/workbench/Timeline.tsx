//! A Conversation's Timeline: everything that has happened to it, in order.
//!
//! Two kinds of Event so far — the Brief and a move — drawn as a list of Events
//! rather than as a Brief with a list under it. The stages after this one put
//! agent output, Question Sets and commits on the same list, and a Timeline
//! built around its first Event would have to be taken apart to hold the second.
//!
//! The Timeline is also where the grilling is started from, because that is
//! where the reason to start it is: the button sits under the Brief it will
//! freeze, at the end of everything that has happened so far, which is exactly
//! where the next thing to happen belongs. Aborting is not in the list — it is
//! not a step in the work but a way of ending it — so it hangs off the header
//! instead, behind a menu, where a destructive action is not one stray click
//! away.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { abortConversation, saveBrief, startGrilling } from "../api/client";
import type {
  BriefEvent,
  BriefSaved,
  ConversationAborted,
  ConversationView,
  GrillingStarted,
  Lifecycle,
  MovedEvent,
} from "../api/types";

/// What each way of being refused a Brief says.
///
/// `Saved` is here for completeness of the mapping and never drawn: nothing is
/// said about an edit that worked, because the Brief reading back as what was
/// written is what says it.
export const BRIEF_REFUSAL: Record<BriefSaved, string> = {
  Saved: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The brief was frozen when grilling started, so it cannot be edited.",
};

/// And each way of being refused a start.
///
/// Every one of them is something different to go and do, which is the whole
/// reason the server names them separately rather than saying "cannot start".
export const GRILL_REFUSAL: Record<GrillingStarted, string> = {
  Started: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "This conversation has already been started.",
  NoGrillingProfile: "Choose a grilling profile first, in the details pane.",
  NoImplementationProfile:
    "Choose an implementation profile first, in the details pane.",
  ProfileBroken:
    "A chosen profile's claude pair is not where it was left, so there is no account to run under.",
  EmptyBrief: "Write the brief first — it is what the grilling starts from.",
  NoBaseCommit: "The repo has nothing to branch from any more.",
  BranchExists: "That branch already exists, and Verkstead did not make it.",
  WorktreeRefused: "Git would not make the worktree. The server log says why.",
};

/// And each way of being refused an abort.
export const ABORT_REFUSAL: Record<ConversationAborted, string> = {
  Aborted: "",
  AlreadyAborted: "",
  NoSuchConversation: "This conversation is gone.",
  WorktreeStuck:
    "The worktree could not be removed, so nothing was changed. The server log says why.",
};

/// What a move reads as. The state moved *to*, said as something that happened.
const MOVED: Record<Lifecycle, string> = {
  Draft: "Went back to drafting",
  Grilling: "Started grilling",
  Direction: "Moved to choosing a direction",
  Implementing: "Started implementing",
  Wrapping: "Moved to wrapping up",
  Done: "Finished",
  Aborted: "Aborted",
};

export function Timeline(props: {
  conversation: ConversationView;
  back: () => void;
  details: () => void;
}): JSX.Element {
  return (
    <>
      <div class="pane-head">
        {/* The way back out of this level, which is the whole of what a narrow
            window offers instead of the pane beside it. Drawn always and hidden
            by the stylesheet where all three panes are on screen at once. */}
        <button type="button" class="pane-back" onClick={props.back}>
          ← Conversations
        </button>
        <h1>{props.conversation.branch}</h1>
        <Actions conversation={props.conversation} />
        <button type="button" class="pane-forward" onClick={props.details}>
          Details →
        </button>
      </div>

      <ol class="timeline">
        <For each={props.conversation.timeline}>
          {(event) => (
            <li class="timeline-event">
              <Switch>
                <Match when={"Brief" in event && event.Brief}>
                  {(brief) => (
                    <Brief id={props.conversation.id} brief={brief()} />
                  )}
                </Match>
                <Match when={"Moved" in event && event.Moved}>
                  {(moved) => <Moved moved={moved()} />}
                </Match>
              </Switch>
            </li>
          )}
        </For>
      </ol>

      {/* After everything that has happened, because it is what happens next.
          Drawn outside the list: it is not an event, and it would be an event
          that moved every time one landed. */}
      <StartGrilling conversation={props.conversation} />
    </>
  );
}

/// A move: the Conversation changing hands, said in a line.
///
/// A line and not a card, because there is nothing to it but the fact and the
/// time — everything a move has to say is already in the two.
function Moved(props: { moved: MovedEvent }): JSX.Element {
  return (
    <p class="moved" classList={{ [props.moved.state.toLowerCase()]: true }}>
      {MOVED[props.moved.state]}
    </p>
  );
}

/// The button that gives a Conversation somewhere to work.
///
/// Drawn only while there is something to start. `ready_to_grill` decides
/// whether it is *offered* rather than whether it is enabled: a conversation
/// that has already started has nothing to press, and one that is not ready is
/// told what is missing rather than handed a dead control. The server checks
/// every one of the conditions again regardless — the page's copy is only as
/// fresh as its last read.
function StartGrilling(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<GrillingStarted | null>(null);

  const start = useMutation(() => ({
    mutationFn: () => startGrilling(props.conversation.id),
    onSuccess: (outcome: GrillingStarted) => {
      if (outcome !== "Started") {
        setRefused(outcome);
        // Refused against a picture of the world this page read a moment ago:
        // reading it again is both the correction and the explanation.
        void queries.invalidateQueries({ queryKey: ["conversation"] });
        void queries.invalidateQueries({ queryKey: ["profiles"] });
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <Show when={props.conversation.state === "Draft"}>
      <div class="start-grilling">
        <Show
          when={props.conversation.ready_to_grill}
          fallback={
            // Deliberately not the details pane's wording. That one is a
            // verdict on the conversation, drawn where the profiles are fixed;
            // this one stands in for the button, and says what would make it
            // appear.
            <p class="note">
              Write the brief and choose both agent profiles, and the grilling
              can start.
            </p>
          }
        >
          <button
            type="button"
            class="start"
            disabled={start.isPending}
            onClick={() => start.mutate()}
          >
            {start.isPending ? "Starting…" : "Start grilling"}
          </button>
          <p class="note">
            This creates the branch and its worktree, and freezes the brief.
          </p>
        </Show>

        <Show when={refused()}>
          {(outcome) => <p class="error">{GRILL_REFUSAL[outcome()]}</p>}
        </Show>
        <Show when={start.isError}>
          <p class="error">
            The grilling could not be started: {start.error?.message}
          </p>
        </Show>
      </div>
    </Show>
  );
}

/// What can be done to the conversation as a whole, rather than to any one
/// event: a menu on the header, holding abort.
///
/// A menu rather than a button, because aborting throws a worktree away and the
/// header is somewhere the human's cursor passes on the way to everything else.
/// Native `details`/`summary`, so it opens, closes and reaches the keyboard
/// without any of that being this component's to get right.
function Actions(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [open, setOpen] = createSignal(false);
  const [refused, setRefused] = createSignal<ConversationAborted | null>(null);

  const abort = useMutation(() => ({
    mutationFn: () => abortConversation(props.conversation.id),
    onSuccess: (outcome: ConversationAborted) => {
      if (outcome === "NoSuchConversation" || outcome === "WorktreeStuck") {
        setRefused(outcome);
        return;
      }

      // Aborted or already aborted: what was asked for holds either way.
      setRefused(null);
      setOpen(false);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <details
      class="conversation-actions"
      open={open()}
      onToggle={(ev) => setOpen(ev.currentTarget.open)}
    >
      <summary aria-label="Conversation actions">⋯</summary>
      <div class="menu">
        <Show
          when={props.conversation.state !== "Aborted"}
          fallback={<p class="note">This conversation has been aborted.</p>}
        >
          <button
            type="button"
            class="abort"
            disabled={abort.isPending}
            onClick={() => abort.mutate()}
          >
            {abort.isPending ? "Aborting…" : "Abort conversation"}
          </button>
          <p class="note">
            Removes the worktree. The branch stays where it is.
          </p>
        </Show>

        <Show when={refused()}>
          {(outcome) => <p class="error">{ABORT_REFUSAL[outcome()]}</p>}
        </Show>
        <Show when={abort.isError}>
          <p class="error">
            The conversation could not be aborted: {abort.error?.message}
          </p>
        </Show>
      </div>
    </details>
  );
}

/// The Brief: the markdown a Conversation starts from, read inline and written
/// inline.
///
/// Inline in the Timeline rather than in the details pane, because there is
/// nothing of it the Timeline does not already show — it *is* its own summary.
///
/// Read as the server rendered it and written as it was typed. The two are one
/// field's worth of markdown either way, and the Brief is the one document on
/// this wire that travels both ways for exactly that reason.
function Brief(props: { id: number; brief: BriefEvent }): JSX.Element {
  const queries = useQueryClient();

  // Whether the Brief is being written rather than read. Its own signal and not
  // "is there a draft": an empty Brief is a perfectly ordinary thing to open the
  // field on, and it is the first thing anyone does with a new Conversation.
  const [editing, setEditing] = createSignal(false);

  // What is being typed. Seeded from the Brief when editing starts rather than
  // kept in step with it, so a Brief that changed underneath is the one that
  // opens in the field.
  const [draft, setDraft] = createSignal("");

  const [refused, setRefused] = createSignal<BriefSaved | null>(null);

  const write = () => {
    setDraft(props.brief.markdown);
    setEditing(true);
  };

  const stop = () => {
    setEditing(false);
    setRefused(null);
  };

  const save = useMutation(() => ({
    mutationFn: (markdown: string) => saveBrief(props.id, markdown),
    onSuccess: (outcome: BriefSaved) => {
      if (outcome !== "Saved") {
        // The draft stands: it is the only copy of what was written, and the
        // human is owed the chance to take it somewhere else.
        setRefused(outcome);
        return;
      }

      setRefused(null);
      setEditing(false);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <article class="brief">
      <div class="event-head">
        <h2>Brief</h2>
        <Show when={!editing()}>
          <button type="button" class="edit-brief" onClick={write}>
            Edit
          </button>
        </Show>
      </div>

      <Show
        when={editing()}
        fallback={
          <Show
            when={props.brief.markdown !== ""}
            fallback={
              <p class="empty">
                Nothing written yet — this is what the grilling starts from.
              </p>
            }
          >
            <div class="brief-body markdown" innerHTML={props.brief.html} />
          </Show>
        }
      >
        <form
          class="edit-brief-form"
          onSubmit={(ev) => {
            ev.preventDefault();
            save.mutate(draft());
          }}
        >
          {/* A copy of what has been typed gives the field its height — see
              `.grow`. */}
          <div class="grow" data-value={draft()}>
            <textarea
              rows="1"
              aria-label="Brief"
              placeholder="What is this piece of work?"
              value={draft()}
              onInput={(ev) => {
                setDraft(ev.currentTarget.value);
                setRefused(null);
              }}
            />
          </div>
          <div class="edit-brief-buttons">
            <button type="submit" disabled={save.isPending}>
              {save.isPending ? "Saving…" : "Save"}
            </button>
            <button type="button" class="cancel" onClick={stop}>
              Cancel
            </button>
          </div>
          <Show when={refused()}>
            {(outcome) => <p class="error">{BRIEF_REFUSAL[outcome()]}</p>}
          </Show>
          <Show when={save.isError}>
            <p class="error">
              The brief could not be saved: {save.error?.message}
            </p>
          </Show>
        </form>
      </Show>
    </article>
  );
}
