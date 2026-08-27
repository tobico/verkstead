//! The attached Diff: what the agent is asking about, shown rather than
//! described.
//!
//! It arrives as HTML the server already rendered — parsed per file,
//! highlighted, and anchored `diff-1`, `diff-2`, … in the order the paths
//! beside it name — so there is nothing here to render and no diff parser in
//! the browser at all. The folds are the browser's own `details`, which is why
//! they work without a line of script.
//!
//! One section, and a block inside it per repository the work may be written in.
//! Which repository a block came out of is drawn over it where the server named
//! one, and a Diff of a single block is the work's own repository and is drawn
//! without a name: the label earns its place when repos mix, exactly as a commit
//! card's does.

import { For, Show } from "solid-js";
import type { JSX } from "solid-js";

import app from "../App.module.css";
import { Switch } from "../Switch";
import type { RepoDiffView } from "../api/types";
import styles from "./Diff.module.css";

/// The attached Diff, and the one setting that governs how it is read.
///
/// The wrap switch sits beside the heading rather than in a settings page
/// somewhere, because this is the only place its answer is visible — and it
/// governs every Diff, not this one, which is why it is remembered on the
/// device instead of per Set.
///
/// Wrapping is a class and nothing more: the Diff arrived rendered, so there is
/// nothing here to render again and the stylesheet is the whole of the change.
export function Diff(props: {
  blocks: RepoDiffView[];
  wrapped: boolean;
  flip: (on: boolean) => void;
}): JSX.Element {
  return (
    <section
      class={props.wrapped ? `${styles.diff} ${styles.wrapped}` : styles.diff}
      id="diff"
    >
      <div class={app.sectionHead}>
        <h2 class={app.sectionHeading}>Diff</h2>
        <Switch label="Word wrap" on={props.wrapped} flip={props.flip} />
      </div>
      <For each={props.blocks}>
        {(block) => (
          <>
            {/* Which repository this is, where more than one is being shown.
                The table of contents says the same word over the same files,
                which is how the two read as one account of the Diff. */}
            <Show when={block.repo}>
              {(repo) => <h3 class={styles.repo}>{repo()}</h3>}
            </Show>
            {/* The per-file anchors are stamped by the renderer, since this
                arrives already rendered. */}
            <div class={styles.diffFiles} innerHTML={block.diff.html} />
          </>
        )}
      </For>
    </section>
  );
}
