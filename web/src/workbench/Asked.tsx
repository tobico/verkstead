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
//! What draws it is [`Sheet`], header and all: the sheet is the whole of what a
//! Set looks like, and this pane is where a Set is read. What is left here is
//! the fetch and the four things a read can come back as — waiting, refused, a
//! record this build cannot draw, and the Set itself.
//!
//! Answering it here ends the wait the session is holding, exactly as answering
//! it on a phone does — both go through the one endpoint, and the Nudge that
//! follows is what brings this pane back saying what was decided.

import { Match, Switch, type JSX } from "solid-js";

import { PaneSticky } from "../Panes";
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
/// which of the two it is drawing.
export function Asked(props: {
  asked: QuestionSetEvent | UnreadableSetEvent;
  back: () => void;
}): JSX.Element {
  const set = useReading(() => ({
    // Under the Set's own id rather than the Event's, so that a Set answered
    // anywhere is the same Set here: the key is what a Nudge invalidates.
    queryKey: ["set", String(props.asked.set_id)],
    queryFn: () => loadSet(String(props.asked.set_id)),

    // Merged into what is already drawn rather than replacing it, which is what
    // keeps a re-read from closing the folds the reader has opened down the
    // attached Diff — see `freshness.ts`.
    freshness: { reconcile: "id" },
  }));

  // What there is of a header before there is a Set to title it with: the way
  // back out of the pane, and nothing else to say yet.
  const head = (
    <PaneSticky>
      <PaneHead back={{ to: "Timeline", go: props.back }} />
    </PaneSticky>
  );

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
      {/* A stored body this build cannot read is the record drawn as itself —
          the narrower match, so it goes first. Both of these draw their own
          header, the way back included: a Set is titled by what it asked. */}
      <Match when={set.data && unreadable(set.data)}>
        {(unreadable) => <Unreadable set={unreadable()} back={props.back} />}
      </Match>
      <Match when={set.data && readable(set.data)}>
        {(set) => (
          // With its table of contents, which takes its shape from the pane's
          // width: the pane caps what it holds at the same 60rem every other
          // column is read at and centres it, so there is a margin for the
          // sidebar to stand in — and where the human has left the pane
          // narrower than that, the nav folds into its bar.
          <Sheet set={set()} back={props.back} />
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
