//! A stack of documents in the details pane, each in a boxed section.
//!
//! What both plan cards open onto. A backlog's card is its entries and a
//! roadmap's card is its stages, and in each case what the card cannot show is
//! what the entries *say*: the documents beside the list — `.tasks/NN-<slug>.md`
//! and `docs/roadmaps/<name>/NN-<slug>.md` — which are the same kind of thing
//! written by forks of the same skill. So they are drawn by the one component,
//! each in the boxed section a set's Preface is drawn in: the heading outside
//! the box, the rendered markdown inside it.
//!
//! Nothing here fetches or parses anything. The documents arrive as HTML the
//! server rendered and sanitized, with any Diagram left as the source block the
//! client-side renderer draws over — the set page's own arrangement, and a
//! stack whose documents hold none never asks for mermaid.

import { For, Show, createEffect, on, onCleanup, type JSX } from "solid-js";

import app from "../App.module.css";
import { drawDiagrams } from "../set/diagrams";
import styles from "./Documents.module.css";

/// One section of the stack: what the list says about the entry, and its
/// document as the file holds it.
export type DocumentSection = {
  /// What the section is reached by, from the table of contents down the
  /// margin. Its own prefix rather than a bare number, because a commit's pane
  /// and a set's page can be open at once and an id names one element in a
  /// document.
  anchor: string;

  /// As the list writes it, zero-padding and all — `01`.
  number: string;

  title: string;

  /// The document, or `null` where there is none to draw.
  html: string | null;

  /// What the box says in that case, which is different for each of them: a
  /// task's file going is the ordinary end of its life, and a roadmap naming a
  /// brief nobody wrote is not.
  missing: string;

  /// Whatever else the heading carries, at the far end of it — a stage's done
  /// state. Nothing on a task: its document being gone is what says it is done,
  /// and the box below already says so.
  mark?: JSX.Element;
};

export function Documents(props: {
  sections: DocumentSection[];
  diagrams: boolean;
}): JSX.Element {
  let block!: HTMLDivElement;

  // On the sections that are in the block rather than on this component's
  // mount: opening a second conversation's backlog is not a second mount, and
  // the markup is assigned into the block the first one built. Following the
  // sections because they are what is being drawn over — assigning the HTML is
  // a render effect, and those are all through before the first of these runs.
  createEffect(
    on(
      () => props.sections,
      () => {
        if (!props.diagrams) {
          return;
        }

        // Stopped when the next stack arrives as much as when the pane goes: a
        // drawing nobody stopped is still watching the colour scheme, and would
        // go on redrawing nodes this block no longer holds.
        onCleanup(drawDiagrams({ root: block }));
      },
    ),
  );

  return (
    <div ref={block}>
      <For each={props.sections}>
        {(section) => (
          /* Named and anchored the way a set's Preface is: the heading is what
             a jump from the table of contents lands on, the id is what it jumps
             to, and the heading stays outside the box, which is what makes the
             two look alike. */
          <section id={section.anchor} class={styles.section}>
            <h2 class={app.sectionHeading}>
              <span class={styles.n}>{section.number}</span>
              <span class={styles.what}>{section.title}</span>
              <Show when={section.mark}>
                {(mark) => <span class={styles.mark}>{mark()}</span>}
              </Show>
            </h2>
            <Show
              when={section.html}
              fallback={<p class={styles.missing}>{section.missing}</p>}
            >
              {(html) => (
                /* Marked as rendered markdown, so the headings, tables and code
                   these documents are written with get the same rules here as
                   they get in a Preface — the box around it is all that is this
                   section's own. */
                <div class={`${styles.document} markdown`} innerHTML={html()} />
              )}
            </Show>
          </section>
        )}
      </For>
    </div>
  );
}
