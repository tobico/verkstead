//! A Conversation's Timeline: everything that has happened to it, in order.
//!
//! One kind of Event so far — the Brief — but drawn as a list of Events rather
//! than as a Brief with a list under it. The stages after this one put agent
//! output, Question Sets and commits on the same list, and a Timeline built
//! around its first Event would have to be taken apart to hold the second.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { saveBrief } from "../api/client";
import type { BriefEvent, BriefSaved, ConversationView } from "../api/types";

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
              </Switch>
            </li>
          )}
        </For>
      </ol>
    </>
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
