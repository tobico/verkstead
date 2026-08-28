//! A card of the agent's own writing: the heading outside the box, the markdown
//! the server rendered inside it.
//!
//! Two sections of the app are this exact thing, and used to be two copies of
//! it: a Set's Preface, and the Message a commit's pane opens with. They are the
//! same kind of thing read the same way — a passage the agent wrote, under a
//! heading the table of contents offers a way to — so they are one component,
//! and the box cannot drift between them any more.
//!
//! Here rather than with either of them, for the reason `Modal` and `Switch` are
//! here: neither the Set page nor the workbench owns it, and a card living with
//! one of the two would be the other one borrowing.
//!
//! Nothing here parses or fetches anything. The markdown arrives as HTML the
//! server rendered and sanitized, and a Diagram in it is left as the source
//! block a caller may draw over — which is what `ref` is for.

import type { JSX } from "solid-js";

import app from "./App.module.css";
import styles from "./Card.module.css";

export function Card(props: {
  /// What the section is reached by, from the table of contents. The caller's
  /// own name for it: a commit's pane and a Set's page can be open at once, and
  /// an id names one element in a document.
  anchor: string;

  /// What the heading says, which is what the nav's line naming it says.
  heading: string;

  /// The markdown, rendered and sanitized by the server on the way out.
  html: string;

  /// The body block, handed back to a caller with a Diagram to draw over it.
  /// A Set's page draws over the whole document instead, and passes none.
  ref?: (block: HTMLDivElement) => void;
}): JSX.Element {
  return (
    /* The heading is what a jump from the table of contents lands on, the id is
       what it jumps to, and the heading stays outside the box — which is what
       makes every card drawn this way look alike.

       The body is marked as rendered markdown, so the agent's headings, tables
       and code get the same rules here as they get inside a Question: the box
       around it is all that is this component's own. */
    <section class={styles.card} id={props.anchor}>
      <h2 class={app.sectionHeading}>{props.heading}</h2>
      <div
        class={`${styles.cardBody} markdown`}
        ref={(block) => props.ref?.(block)}
        innerHTML={props.html}
      />
    </section>
  );
}
