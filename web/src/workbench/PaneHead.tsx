//! The bar across the top of a workbench pane: what the pane is called, the way
//! back out to the pane it was entered from, and the way out of an open Event.
//!
//! Seven panes wrote this out by hand — the same `div`, the same button drawn
//! twice under two class names, the same "← Timeline" spelled six times — and
//! then hung their own controls in the row: the record switch, the *Blocked on
//! you* badge, the ⋯ menus, the way on to the details. What they were repeating
//! was chrome rather than content, so the chrome is here and the row is still
//! theirs: a pane hands in its title, says which pane it is entered from, and
//! puts whatever else it needs in the header inside the tags.
//!
//! The order is the one every pane already drew and the one the header reads
//! in: the way back over the top, the title, the pane's own controls after it,
//! and Close at the end.

import { Show, type JSX } from "solid-js";

import styles from "./PaneHead.module.css";

export function PaneHead(props: {
  /// The pane this one was entered from, named as the way back reads it — "←
  /// Timeline" — and what pressing it does. Absent on the sidebar, which is the
  /// level everything else is entered from.
  back?: { to: string; go: () => void };
  /// What the pane is called, drawn as its `<h1>`. Absent where the pane is
  /// titled by what it holds rather than by a word of its own.
  title?: JSX.Element;
  /// A class of the caller's on that heading, for the pane whose title is a
  /// mark rather than words — the `class` prop the menu and the modal already
  /// take, styled by whoever passes it and never here.
  heading?: string;
  /// The pane's own controls, standing in the header row after the title.
  children?: JSX.Element;
  /// The way out of the open Event, back to what the conversation is. Absent
  /// where the pane has no Event to close or has given the slot to a control of
  /// its own.
  close?: () => void;
}): JSX.Element {
  return (
    <div class={styles.head}>
      <Show when={props.back}>
        {(back) => (
          <button type="button" class={styles.back} onClick={() => back().go()}>
            ← {back().to}
          </button>
        )}
      </Show>

      <Show when={props.title !== undefined}>
        <h1 class={props.heading}>{props.title}</h1>
      </Show>

      {props.children}

      <Show when={props.close}>
        <button
          type="button"
          class={styles.close}
          onClick={() => props.close?.()}
        >
          Close
        </button>
      </Show>
    </div>
  );
}
