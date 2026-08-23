//! One Question Set, opened: the whole document in the details pane, answerable
//! where it stands.
//!
//! Fetched here rather than carried by the Conversation, for the reason the
//! Capture is: the two are different sizes. The Timeline carries the table of
//! what was asked against what was decided and is re-read every time the page
//! hears the world moved; the document — the Preface, every Option of every
//! Question, and the whole uncommitted Diff of the repository it was asked from —
//! is read when somebody opens the one Event it belongs to.
//!
//! What draws it is [`Sheet`], which is what the standalone Set page draws too.
//! The rendering is not this stage's to rewrite: a Set reached through its
//! Conversation is the same Set, and a second copy of the drawing would be a
//! second reading of one decision.
//!
//! Answering it here ends the wait the session is holding, exactly as answering
//! it on a phone does — both go through the one endpoint, and the Nudge that
//! follows is what brings this pane back saying what was decided.

import { Match, Switch, type JSX } from "solid-js";

import { loadSet } from "../api/client";
import type { QuestionSetEvent } from "../api/types";
import { useReading } from "../freshness";
import { Sheet } from "../set/Sheet";

export function Asked(props: {
  asked: QuestionSetEvent;
  back: () => void;
  close: () => void;
}): JSX.Element {
  const set = useReading(() => ({
    // The same key the standalone page reads a Set under, so answering it in
    // either place puts the other right.
    queryKey: ["set", String(props.asked.set_id)],
    queryFn: () => loadSet(String(props.asked.set_id)),

    // And the same merge, for the same fold: this pane draws the same Sheet,
    // attached Diff and all — see the standalone page.
    freshness: { reconcile: "id" },
  }));

  const head = (
    <div class="pane-head">
      <button type="button" class="pane-back" onClick={props.back}>
        ← Timeline
      </button>
      {/* The way back to what the conversation is, which is what this pane
          shows when no event is open. */}
      <button type="button" class="close-event" onClick={props.close}>
        Close
      </button>
    </div>
  );

  return (
    <Switch>
      <Match when={set.isPending}>
        {head}
        <p class="empty">Loading…</p>
      </Match>
      <Match when={set.isError}>
        {head}
        <p class="error">Could not read this set: {set.error?.message}</p>
      </Match>
      <Match when={set.data}>
        {(set) => (
          // No table of contents: that is a description of a column the whole
          // window wide, and this is a column beside two others.
          <Sheet set={set()} lead={head} contents={false} />
        )}
      </Match>
    </Switch>
  );
}
