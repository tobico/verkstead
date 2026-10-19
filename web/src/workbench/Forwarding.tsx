//! What this device is forwarding, at the foot of the sidebar: **Forwarding 3
//! ports** beside *Show archived*, and under a press the Forwards themselves.
//!
//! A **Forward** is a port a terminal on another device of the cluster is
//! listening on, held on this machine's own `localhost` at the same number from
//! the first time the terminal's tab is opened here until the server stops or
//! the terminal ends. The server holds them — see `forwarding.rs` — and this
//! says what it holds: nothing here starts one. The one thing it does is the ×
//! at the end of each row, which ends that Forward on this device and keeps it
//! ended until the port closes, the terminal's tab is opened here again, or
//! this device's server restarts.
//!
//! Drawn only while there is something to say, forwarding, skipped or
//! unreachable: a foot that read *Forwarding 0 ports* on every machine that has
//! never opened a member's terminal would be a line about nothing on nearly
//! every page.
//!
//! Not drawn in the sidebar's collapsed strip, where the foot it stands in is
//! blank paper; it keeps its room there all the same, wrapped under the switch
//! where the expanded pane would wrap it. See `Archived.module.css`.
//!
//! The popup hangs over the item rather than under it, the item standing at the
//! bottom of the pane; it is the one menu the app has, so it closes the ways
//! every menu does and hands the focus back to the item on Escape. A × is not
//! one of those ways: it takes its row away and leaves the rest standing, so
//! several can be ended in one visit — and the item goes with the last of them.

import { faXmark } from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, type JSX } from "solid-js";

import { Icon } from "../Icon";
import { Menu } from "../Menu";
import { dismissForward, loadForwards } from "../api/client";
import type { ForwardSkip, ForwardView, ForwardsView } from "../api/types";
import { osIcon } from "../devices";
import { useReading } from "../freshness";
import styles from "./Forwarding.module.css";

/// What a skipped Forward reads, off why it was skipped.
const SKIPPED: Record<ForwardSkip, string> = {
  port_busy: "port busy here",
  not_permitted: "not permitted here",
};

/// The item, and the popup it opens.
export function Forwarding(): JSX.Element {
  /// This device's own, whichever page is around it: a Forward is a listener on
  /// this machine, so there is no member to ask. Re-read on the `forwards`
  /// Nudge, which the server announces on every change to the register.
  ///
  /// Merged by the port — the one thing a row is told apart by on the screen.
  /// Two members' Forwards can share one, the second skipped as busy, and the
  /// merge keeps duplicates in their order.
  const reading = useReading(() => ({
    queryKey: ["forwards"],
    queryFn: loadForwards,
    freshness: { reconcile: "port" },
  }));

  const forwards = (): ForwardView[] => reading.data?.forwards ?? [];

  return (
    <Show when={forwards().length > 0}>
      <Menu
        class={styles.forwarding!}
        up
        name="Forwarded ports"
        trigger={
          <>
            Forwarding {forwards().length}{" "}
            {forwards().length === 1 ? "port" : "ports"}
          </>
        }
      >
        {() => (
          <For each={forwards()}>{(forward) => <Row forward={forward} />}</For>
        )}
      </Menu>
    </Show>
  );
}

/// What an unreachable Forward reads: its member is not answering.
const UNREACHABLE = "device unreachable";

/// One Forward: the port, the device it reaches and the Conversation the
/// terminal is on — and the × that ends it.
///
/// A link where it is forwarding, opening `localhost` at that port in a tab of
/// its own — which is the server on the member, carried across. A skipped one
/// is not a link: `localhost` at that number is whatever this machine already
/// had there, which is exactly what the row is not. Nor is an unreachable one:
/// the port is still held here, but a page opened on it would fail, and the
/// row is there to say why.
///
/// The × is a control of its own beside the row rather than inside it, so the
/// rest of the row is still one target for the link. A press takes the row
/// away at once, and the server's `forwards` Nudge says the same a moment
/// later. A × pressed with the focus on it hands the focus to the next row's ×,
/// or the one before where it was the last, so a hand on the keyboard stays in
/// the card rather than being dropped onto the page with the row.
function Row(props: { forward: ForwardView }): JSX.Element {
  const queries = useQueryClient();

  let row!: HTMLDivElement;

  /// Where the focus goes once the row has gone — read at the press, while the
  /// row is still there to look either side of, and only where the focus was
  /// on this row: a × nobody was on is no reason to move it.
  let handing: HTMLButtonElement | null = null;

  const dismiss = useMutation(() => ({
    mutationFn: () => dismissForward(props.forward),
    onSuccess: () => {
      queries.setQueryData<ForwardsView>(["forwards"], (read) =>
        read
          ? {
              forwards: read.forwards.filter(
                (held) => !same(held, props.forward),
              ),
            }
          : read,
      );
      void queries.invalidateQueries({ queryKey: ["forwards"] });

      if (handing?.isConnected) handing.focus();
      handing = null;
    },
    onError: () => {
      handing = null;
    },
  }));

  const press = (): void => {
    const neighbour = row.nextElementSibling ?? row.previousElementSibling;

    handing = row.contains(document.activeElement)
      ? (neighbour?.querySelector<HTMLButtonElement>("button") ?? null)
      : null;

    dismiss.mutate();
  };

  const skipped = (): string | null => {
    const standing = props.forward.standing;

    switch (standing.kind) {
      case "forwarding":
        return null;
      case "skipped":
        return SKIPPED[standing.reason];
      case "unreachable":
        return UNREACHABLE;
    }
  };

  const lines = (): JSX.Element => (
    <>
      <span class={styles.port}>localhost:{props.forward.port}</span>
      {/* A line of its own rather than a tail on the port's: the card is as
          narrow as the sidebar, and every line of a row is cut short before it
          wraps — which a reason must never be. */}
      <Show when={skipped()}>
        {(reason) => <span class={styles.reason}>{reason()}</span>}
      </Show>
      <span class={styles.device}>
        <Icon of={osIcon(props.forward.os)} class={styles.os} />
        {props.forward.name}
      </span>
      <span class={styles.title}>
        {props.forward.title ?? `Conversation ${props.forward.conversation}`}
      </span>
    </>
  );

  return (
    <div role="none" class={styles.row} ref={row}>
      <Show
        when={!skipped()}
        fallback={
          <span role="menuitem" aria-disabled="true" class={styles.skipped}>
            {lines()}
          </span>
        }
      >
        <a
          role="menuitem"
          href={`http://localhost:${props.forward.port}/`}
          target="_blank"
          rel="noreferrer"
        >
          {lines()}
        </a>
      </Show>

      {/* Called by the port it ends: a row of these all read aloud as one
          word would say nothing about which. */}
      <button
        type="button"
        role="menuitem"
        class={styles.stop}
        aria-label={`Stop forwarding localhost:${props.forward.port}`}
        disabled={dismiss.isPending}
        onClick={press}
      >
        <Icon of={faXmark} />
      </button>
    </div>
  );
}

/// Whether two rows are the same Forward: the member, the Conversation, the
/// terminal and the port, which is what the server keys one by.
function same(one: ForwardView, other: ForwardView): boolean {
  return (
    one.port === other.port &&
    one.device === other.device &&
    one.conversation === other.conversation &&
    one.terminal === other.terminal
  );
}
