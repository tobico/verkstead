//! The one setting that is about the list of Conversations rather than about
//! the rest of Verkstead: whether the ones the human has archived are drawn in
//! it.
//!
//! It stood at the foot of the sidebar, that being the only page with a list to
//! put it under. There are two places now — the sidebar, and the compose page
//! standing without one while there is nothing to list, where it is pinned to
//! the corner and drawn only where there is something behind it to bring back
//! (see `zero.ts`) — so it is here rather than written a second time over there.
//!
//! Where it sits is the pane's rather than this component's: it wears the
//! frame's `paneFoot`, so whichever pane it is handed to sticks it against its
//! bottom edge with the paper the content passes behind.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, type JSX } from "solid-js";

import shell from "../Panes.module.css";
import { Switch as Toggle } from "../Switch";
import { showArchived, showingArchived } from "../api/client";
import { useReading } from "../freshness";
import { ErrorLine } from "../notices";
import styles from "./Archived.module.css";

/// A switch rather than something that presses, because it is a state the list
/// is in rather than something to do to it. *Show archived* rather than the
/// whole sentence it could be: it stands under the list of conversations, so
/// what else it could be showing does not have to be said.
/// `children` is what stands at the other end of the line from the switch: the
/// sidebar puts what this device is forwarding there (see `Forwarding.tsx`),
/// and the compose page's copy, drawn only while there is no list, puts
/// nothing — with no Conversation there is no terminal to forward from.
export function ShowArchived(props: { children?: JSX.Element }): JSX.Element {
  const queries = useQueryClient();

  /// The server's answer rather than this device's: the choice is the human's,
  /// so a phone opened afterwards is looking at the same list.
  const showing = useReading(() => ({
    queryKey: ["conversations", "archived"],
    queryFn: showingArchived,

    // Two booleans, so there is nothing in it to hold on to and nothing to
    // match up: what a re-read lands on is the whole payload either way.
    freshness: { reconcile: "id" } as const,
  }));

  const flip = useMutation(() => ({
    mutationFn: (on: boolean) => showArchived(on),
    onSuccess: () => {
      // The list itself and the switch under it: what is drawn changes with the
      // setting, which is the entire point of it. The other devices hear the
      // same news as a Nudge.
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  /// Where the switch stands: the position asked for while that is in flight,
  /// and the server's the rest of the time. A switch that snapped back to the
  /// old position for the length of a round trip would read as a press that
  /// failed.
  const on = (): boolean =>
    flip.isPending ? (flip.variables ?? false) : (showing.data?.showing ?? false);

  return (
    <div class={`${styles.showArchived} ${shell.paneFoot}`}>
      <div class={styles.line}>
        <Toggle
          label="Show archived"
          on={on()}
          disabled={showing.isPending || flip.isPending}
          flip={(wanted) => flip.mutate(wanted)}
        />
        {props.children}
      </div>
      <Show when={flip.isError}>
        <ErrorLine>
          The setting could not be saved: {flip.error?.message}
        </ErrorLine>
      </Show>
    </div>
  );
}
