//! One section of the settings page as the page lists it: a single line, the
//! section's name at the near end and how it stands at the far one.
//!
//! **One line rather than a paragraph.** The page is a list of places to go, and
//! what somebody scanning it is after is which section and whether anything in
//! it wants them — so the card says the section's state in a few words and the
//! pane it opens says the rest. A sentence on the card was read once and read
//! past forever after, and a page of them read as a form rather than a list.
//!
//! **And the warning in the same line**, in the stopped colour, where a section
//! is waiting on the human — no GitHub token, no sccache, a bind the server
//! cannot see. Said in the fewest words that name it, the pane being where why
//! is explained.
//!
//! Every settings section's card is one of these, so they are one height, one
//! dress and one rhythm down the page; what each section decides is only its
//! title and what its line says. See `SettingsPage.tsx` for the order.

import type { JSX } from "solid-js";

import { CardButton } from "../CardButton";
import styles from "./SettingsCard.module.css";

export function SettingsCard(props: {
  /// The section's name, drawn as its heading.
  title: string;
  /// How it stands, in a few words — cut with an ellipsis where the pane is too
  /// narrow for them, the title never being what gives way.
  status: JSX.Element;
  /// Whether that line is a warning: something in the section is waiting on the
  /// human.
  warn?: boolean;
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
  /// The section's own class on the card, which is what its tests find it by.
  class?: string;
}): JSX.Element {
  return (
    <CardButton
      as="article"
      class={[styles.card, props.class].filter(Boolean).join(" ")}
      open={props.open}
      press={props.press}
    >
      <h2>{props.title}</h2>
      <span
        class={styles.status}
        classList={{ [styles.warn!]: props.warn === true }}
      >
        {props.status}
      </span>
    </CardButton>
  );
}
