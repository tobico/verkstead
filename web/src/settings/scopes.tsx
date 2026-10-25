//! What a GitHub token has to be able to do, written once for the two places
//! one is asked for.
//!
//! **Said before a token is pasted, rather than after it is saved.** Each of
//! those two places already draws what GitHub said about the token that *was*
//! saved — the account it authenticates as, and a scope it turns out not to
//! have. That is a different thing: it is about one token, it only appears
//! after a save, and it names what to do about it. This is the list to tick
//! against on GitHub's own form. The settings page draws it whether or not
//! anything has been saved; the wizard draws GitHub's answer in its place once
//! there is one, as its boards do.
//!
//! **Why Verkstead says it at all.** The push that needs these scopes is made
//! inside the sandbox by a session, so a refusal for a missing one comes back
//! as a failed `git push` rather than as a sentence naming the box that was
//! left unticked. Reporting that push stays the session's, because the push is
//! the session's to make; the note is the half that is Verkstead's, and saying
//! it once where the token is pasted is cheaper than every session working it
//! out afterwards.
//!
//! **One module because it is one sentence.** The wizard's git step and the
//! settings page's token pane both draw this component — two places saying the
//! same thing in two wordings is two things to keep true.

import type { JSX } from "solid-js";

import styles from "./scopes.module.css";

/// What a token needs, as a block to draw under a *GitHub token* heading: a
/// boxed title, a *Classic* line of the scopes as chips, and a *Fine-grained*
/// line naming the permissions.
///
/// A `class` of the parent's rides along the way the notices take one: the two
/// places this stands in space their blocks differently, and that is theirs to
/// say rather than this module's.
export function TokenScopes(props: { class?: string }): JSX.Element {
  return (
    <div class={[styles.scopes, props.class].filter(Boolean).join(" ")}>
      <p class={styles.title}>Tick these on GitHub before pasting</p>
      <ul>
        <li>
          <span class={styles.flavour}>Classic</span>
          <span class={styles.chips}>
            <code>repo</code>
            <code>workflow</code>
            <code>gist</code>
          </span>
        </li>
        <li>
          <span class={styles.flavour}>Fine-grained</span>
          <span class={styles.wanted}>
            Contents, Pull requests, Issues, Workflows to write; Actions to
            read
          </span>
        </li>
      </ul>
    </div>
  );
}
