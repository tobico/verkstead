//! The bar across the top of a workbench pane: what the pane is called, and the
//! way back out to the pane it was entered from.
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
//! in: the way back over the top, the title, and the pane's own controls after
//! it. There is no Close: a details pane is left by opening something else or by
//! the "← Timeline" a narrow window draws, and a header carrying both read as
//! one row asking the same question twice.
//!
//! And it wears one of the frame's own names, as the way back out does: the head
//! is the band the window's controls are drawn over in the app, and which head is
//! under them is a fact about the layout rather than about any pane. So the frame
//! is what pads a head clear of them — see `paneHead` in `Panes.module.css` —
//! and this says no more about it than that it is a head.
//!
//! **And it is the region the window is dragged by**, which is what a window with
//! no title bar has instead of one (ADR-0020). That is a rule in this component's
//! own stylesheet and nothing here: `-webkit-app-region` is a property only a
//! frameless Electron window has ever read, so the head a browser is handed is
//! the head it was before any of it.
//!
//! **The one part of it that is not inert outside the app is the selection**, so
//! that part is the only part this asks about. A head the window moves by cannot
//! also be a head a pointer sweeps a selection across — but `user-select` is an
//! ordinary property every browser honours, and a phone on the tailnet or whoever
//! opens a standalone share has no window to move and every reason to copy the
//! name of what they are reading. So the giving-up is worn where the bridge is,
//! the way the bare drag bar is drawn where it is — see `DragBar.tsx`.

import { Show, type JSX } from "solid-js";

import styles from "./PaneHead.module.css";
import shell from "../Panes.module.css";
import { bridge } from "../settings/bridge";

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
}): JSX.Element {
  // Read once rather than followed, the way the bare drag bar and the Desktop
  // section of the settings read it: a preload is there before the document is,
  // so nothing can arrive or leave while a page is open.
  const app = bridge() !== null;

  return (
    <div
      class={[styles.head, shell.paneHead, app ? styles.inApp : undefined]
        .filter(Boolean)
        .join(" ")}
    >
      <Show when={props.back}>
        {(back) => (
          <button
            type="button"
            class={`${styles.back} ${shell.paneBack}`}
            onClick={() => back().go()}
          >
            ← {back().to}
          </button>
        )}
      </Show>

      <Show when={props.title !== undefined}>
        <h1 class={props.heading}>{props.title}</h1>
      </Show>

      {props.children}
    </div>
  );
}
