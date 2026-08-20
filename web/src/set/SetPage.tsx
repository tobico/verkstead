//! The set view: one Question Set as a page of its own, reached by its id.
//!
//! Everything about drawing one is [`Sheet`]'s — the same rendering the details
//! pane of a Timeline reaches through the Conversation the Set was asked from.
//! What is this page's own is the two things a page has that a pane does not:
//! the id comes out of the URL, and there is a list to go back to.

import { A, useParams } from "@solidjs/router";
import { useQuery } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { Match, Switch } from "solid-js";

import { RefusedError, loadSet } from "../api/client";
import type { SetView } from "../api/types";
import { Sheet } from "./Sheet";

/// One Question Set, as the URL names it.
export function SetPage(): JSX.Element {
  const params = useParams<{ id: string }>();

  const set = useQuery(() => ({
    queryKey: ["set", params.id],
    queryFn: () => loadSet(params.id),
  }));

  return (
    <Switch>
      {/* Pending rather than fetching: the fallback belongs to the first load
          alone. */}
      <Match when={set.isPending}>
        <p class="empty">Loading…</p>
      </Match>
      <Match when={set.isError && absent(set.error)}>
        <p class="empty">No such Set.</p>
      </Match>
      <Match when={set.isError}>
        <p class="error">Could not read the Set: {set.error?.message}</p>
      </Match>
      <Match when={set.data}>
        {(set) => <Sheet set={set()} lead={<Back set={set()} />} />}
      </Match>
    </Switch>
  );
}

/// Back to the list this Set is on: a settled one is off the pending list for
/// good and lives in the Archive, so that is where reading it leads back to.
///
/// The page's own and not the sheet's: a Set reached through its Conversation
/// came from a Timeline, and leads back there instead.
function Back(props: { set: SetView }): JSX.Element {
  const decided = () => !("Waiting" in props.set.standing);

  return (
    <A href={decided() ? "/archive" : "/pending"} class="back">
      {decided() ? "← Archive" : "← Pending"}
    </A>
  );
}

/// Whether there is simply no such Set — which the server says with a 404, and
/// which is a page to draw rather than a failure to report. An id that is not a
/// number gets the same answer, because it cannot name a Set either.
function absent(error: Error | null): boolean {
  return error instanceof RefusedError && error.status === 404;
}
