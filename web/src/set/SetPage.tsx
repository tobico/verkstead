//! The set view: one Question Set as a page of its own, reached by its id.
//!
//! Everything about drawing one is [`Sheet`]'s — the same rendering the details
//! pane of a Timeline reaches through the Conversation the Set was asked from.
//! What is this page's own is the two things a page has that a pane does not:
//! the id comes out of the URL, and there is a list to go back to.

import { A, useParams } from "@solidjs/router";
import type { JSX } from "solid-js";
import { Match, Switch } from "solid-js";

import { RefusedError, loadSet } from "../api/client";
import type { SetReading, SetView, UnreadableSet } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import styles from "./SetPage.module.css";
import { Sheet } from "./Sheet";
import { Unreadable } from "./Unreadable";

/// One Question Set, as the URL names it.
export function SetPage(): JSX.Element {
  const params = useParams<{ id: string }>();

  const set = useReading(() => ({
    queryKey: ["set", params.id],
    queryFn: () => loadSet(params.id),

    // Merge each read into the Set already drawn rather than replacing it, as
    // the workbench does its Conversation. A Set cannot be frozen — its
    // answers change — so it is re-read on every Nudge, and this is what keeps
    // a re-read from disturbing what the reader holds over the page: the
    // attached Diff's markup fills an `innerHTML` that compiles to an
    // unguarded effect over the query's data, and reassigning it would close
    // every per-file fold. Merged, an unchanged string is left alone and the
    // folds stand.
    //
    // The `id` is a level down inside the reading — the payload says which of
    // the two kinds it is first — so the merge matches by structure here, which
    // is what it does for every element the key is absent from. That is sound
    // for this payload: a Set does not change from readable to unreadable while
    // somebody is looking at it, and everything under the one key it does carry
    // merges the way it always did.
    freshness: { reconcile: "id" },
  }));

  return (
    <Switch>
      {/* Pending rather than fetching: the fallback belongs to the first load
          alone. */}
      <Match when={set.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={set.isError && absent(set.error)}>
        <Empty>No such Set.</Empty>
      </Match>
      <Match when={set.isError}>
        <ErrorLine>Could not read the Set: {set.error?.message}</ErrorLine>
      </Match>
      {/* A Set whose stored body this build cannot read is a page of its own
          rather than a failure: the record is there, and this is what there is
          of it. Drawn before the readable one because it is the narrower
          match. */}
      <Match when={set.data && unreadable(set.data)}>
        {(unreadable) => (
          <Unreadable set={unreadable()} lead={<Back to={unreadable().conversation} />} />
        )}
      </Match>
      <Match when={set.data && readable(set.data)}>
        {(set) => <Sheet set={set()} lead={<Back to={set().conversation} />} />}
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

/// Back to the Conversation this Set was asked from, which is where it lives:
/// settled or waiting, a Set is an Event on one Timeline and there is nowhere
/// else for reading it to lead.
///
/// The page's own and not the sheet's. In the workbench this same sheet is the
/// details pane of the Timeline Event it belongs to, and the way back out of
/// that is the pane's header rather than a link to the page it is already on.
function Back(props: { to: number }): JSX.Element {
  return (
    <A href={`/conversations/${props.to}`} class={styles.back}>
      ← Conversation
    </A>
  );
}

/// Whether there is simply no such Set — which the server says with a 404, and
/// which is a page to draw rather than a failure to report. An id that is not a
/// number gets the same answer, because it cannot name a Set either.
function absent(error: Error | null): boolean {
  return error instanceof RefusedError && error.status === 404;
}
