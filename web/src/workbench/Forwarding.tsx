//! What this device is forwarding, at the foot of the sidebar: **Forwarding 3
//! ports** beside *Show archived*, and under a press the Forwards themselves.
//!
//! A **Forward** is a port a terminal on another device of the cluster is
//! listening on, held on this machine's own `localhost` at the same number for
//! as long as the terminal's tab is open here. The server holds them — see
//! `forwarding.rs` — and this only says what it holds: nothing here starts one,
//! and nothing stops one either. The tab is the stop.
//!
//! Drawn only while there is something to say, forwarding or skipped: a foot
//! that read *Forwarding 0 ports* on every machine that has never opened a
//! member's terminal would be a line about nothing on nearly every page.
//!
//! The popup hangs over the item rather than under it, the item standing at the
//! bottom of the pane; it is the one menu the app has, so it closes the ways
//! every menu does and hands the focus back to the item on Escape.

import { For, Show, type JSX } from "solid-js";

import { Icon } from "../Icon";
import { Menu } from "../Menu";
import { loadForwards } from "../api/client";
import type { ForwardSkip, ForwardView } from "../api/types";
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

/// One Forward: the port, the device it reaches and the Conversation the
/// terminal is on.
///
/// A link where it is forwarding, opening `localhost` at that port in a tab of
/// its own — which is the server on the member, carried across. A skipped one
/// is not a link: `localhost` at that number is whatever this machine already
/// had there, which is exactly what the row is not.
function Row(props: { forward: ForwardView }): JSX.Element {
  const skipped = (): ForwardSkip | null =>
    props.forward.standing.kind === "skipped"
      ? props.forward.standing.reason
      : null;

  const lines = (): JSX.Element => (
    <>
      <span class={styles.port}>localhost:{props.forward.port}</span>
      {/* A line of its own rather than a tail on the port's: the card is as
          narrow as the sidebar, and every line of a row is cut short before it
          wraps — which a reason must never be. */}
      <Show when={skipped()}>
        {(reason) => <span class={styles.reason}>{SKIPPED[reason()]}</span>}
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
  );
}
