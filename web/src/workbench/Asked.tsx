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
import type {
  QuestionSetEvent,
  SetReading,
  SetView,
  UnreadableSet,
  UnreadableSetEvent,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Sheet } from "../set/Sheet";
import { Unreadable } from "../set/Unreadable";
import { PaneHead } from "./PaneHead";

/// Either row a Set gets on the Timeline. What they have in common is the only
/// thing this pane needs — which Set to fetch — and what comes back is what says
/// which of the two it is drawing, exactly as it does on the standalone page.
export function Asked(props: {
  asked: QuestionSetEvent | UnreadableSetEvent;
  back: () => void;
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

  // No title: the Set draws its own heading, and a pane titled over the top of
  // it would name the same thing twice.
  const head = <PaneHead back={{ to: "Timeline", go: props.back }} />;

  return (
    <Switch>
      <Match when={set.isPending}>
        {head}
        <Empty>Loading…</Empty>
      </Match>
      <Match when={set.isError}>
        {head}
        <ErrorLine>Could not read this set: {set.error?.message}</ErrorLine>
      </Match>
      {/* A stored body this build cannot read is the record drawn as itself,
          the same one the standalone page draws — the narrower match, so it
          goes first. */}
      <Match when={set.data && unreadable(set.data)}>
        {(unreadable) => <Unreadable set={unreadable()} lead={head} />}
      </Match>
      <Match when={set.data && readable(set.data)}>
        {(set) => (
          // With its table of contents, drawn from the pane's width rather than
          // the window's: the pane caps what it holds at the same 60rem every
          // other column is read at and centres it, so there is a margin here
          // for the sidebar to stand in again — and where the human has left
          // the pane narrower than that, the nav folds into its bar exactly as
          // it does on a narrow window.
          <Sheet set={set()} lead={head} contents="pane" />
        )}
      </Match>
    </Switch>
  );
}

/// The Set inside a reading this build could render, and nothing where it could
/// not.
function readable(reading: SetReading): SetView | undefined {
  return "Set" in reading ? reading.Set : undefined;
}

/// And the record inside one it could not.
function unreadable(reading: SetReading): UnreadableSet | undefined {
  return "Unreadable" in reading ? reading.Unreadable : undefined;
}
