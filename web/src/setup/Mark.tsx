//! Whether a thing the wizard is waiting on stands met, as one mark.
//!
//! Two places draw it — a step's heading in [`./SetupPage.tsx`](./SetupPage.tsx)
//! and every row of the dependencies step — and they are the same question about
//! two sizes of thing, so they are one mark rather than two that would drift.
//!
//! A tick where it is met and an empty ring where it is not: the shapes a pull
//! request's checks are drawn with, for the same reason — the ring is what a
//! tick is cut into once something has happened.

import { faCheck } from "@fortawesome/free-solid-svg-icons";
import { Match, Switch, type JSX } from "solid-js";

import { Icon } from "../Icon";
import styles from "./Mark.module.css";

/// How a thing the wizard is waiting on stands.
export type Standing =
  /// It is there, or it is done.
  | "met"
  /// It is not, and somebody is going to do something about it.
  | "waiting";

/// One of the two, as a mark.
export function Mark(props: { standing: Standing }): JSX.Element {
  return (
    <Switch
      fallback={
        <span class={styles.waiting} aria-label="not yet" role="img" />
      }
    >
      <Match when={props.standing === "met"}>
        <Icon of={faCheck} label="done" class={styles.done} />
      </Match>
    </Switch>
  );
}
