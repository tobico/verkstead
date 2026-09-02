//! A name held to one line, cut at the front where it does not fit, with the
//! whole of it under the pointer.
//!
//! What is truncated here is branch names, in the two places a Conversation is
//! called by one: the row in the sidebar and the header of the pane it opens.
//! They are the one name said twice, so they are cut the same way.
//!
//! **The front and not the end**, which is the opposite of what a browser does
//! by default. A branch name's distinctive half is its tail —
//! `timeline-pinned-polish` and `timeline-own-commits` share everything but it
//! — so a line cut at the end is a column of names that all read the same.
//! `…pinned-polish` says which one this is; `timeline-pinn…` says almost
//! nothing.
//!
//! One line and not a wrap, because both sites have something standing beside
//! the name: the Repo understated after it, and — in the pane header — the
//! controls at the far end of the row, which a title growing downwards used to
//! push onto a line of their own.
//!
//! The whole name is in the native `title`, which is what a cut name owes its
//! reader. Nothing is owed a screen reader here: neither site is read from this
//! element — the sidebar's card is labelled with the sentence in
//! `Conversations.tsx`, and the pane's heading is read as everything under it
//! run together, cut or not.

import type { JSX } from "solid-js";

import styles from "./Truncated.module.css";

export function Truncated(props: {
  /// The name to draw, which is also what the tooltip holds whole.
  text: string;
  /// The caller's own class on the same element — the weight, the size and the
  /// ink are the site's to say, and the cutting is this component's.
  class?: string;
}): JSX.Element {
  return (
    <span
      class={
        props.class === undefined
          ? styles.truncated
          : `${styles.truncated} ${props.class}`
      }
      title={props.text}
    >
      {/* The isolate that keeps the name itself the right way round inside a
          right-to-left line — see the stylesheet. */}
      <bdi>{props.text}</bdi>
    </span>
  );
}
