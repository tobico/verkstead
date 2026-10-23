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
//! Which repository a block came out of is drawn over it wherever the server
//! named one — which is everywhere except the conversation's own repository
//! drawn as the whole of the Diff, an unlabelled block meaning the work's own
//! repo already. A companion's block is named however alone it is, exactly as a
//! commit card is.
//!
//! A picture file in it is drawn rather than read: the renderer leaves a
//! placeholder per side and `pictures.ts` fetches the bytes into it as its fold
//! scrolls near.

import { For, Show, createEffect, on, onCleanup } from "solid-js";
import type { JSX } from "solid-js";

import app from "../App.module.css";
import { Switch } from "../Switch";
import type { RepoDiffView } from "../api/types";
import styles from "./Diff.module.css";
import { drawPictures, type Reader } from "./pictures";

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
  /// How a picture in it is fetched, and none where there is no server to ask —
  /// a share's, where every picture falls back to what the patch holds.
  read?: Reader;
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
            {/* Which repository this is, wherever the server named one. The
                table of contents says the same word over the same files, which
                is how the two read as one account of the Diff. */}
            <Show when={block.repo}>
              {(repo) => <h3 class={styles.repo}>{repo()}</h3>}
            </Show>
            {/* The per-file anchors are stamped by the renderer, since this
                arrives already rendered. */}
            <div
              class={styles.diffFiles}
              innerHTML={block.diff.html}
              ref={(files) => pictured(files, () => block.diff.html, props.read)}
            />
          </>
        )}
      </For>
    </section>
  );
}

/// Draw the pictures in a block of injected markup, and again whenever the
/// markup is assigned afresh: a new assignment is new placeholders.
///
/// Following the HTML rather than the mount, for the reason the commit pane's
/// Message does: assigning it is a render effect, and those are all through
/// before this runs.
export function pictured(
  root: HTMLElement,
  html: () => string,
  read: Reader | undefined,
): void {
  createEffect(on(html, () => onCleanup(drawPictures({ root, read }))));
}
