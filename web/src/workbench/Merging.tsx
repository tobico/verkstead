//! The mark a pull request that will not merge is said in.
//!
//! One shape and one meaning, unlike the three beside it: what is drawn is a
//! conflict, and nothing else here is drawn at all. A pull request that merges
//! cleanly is what a pull request is supposed to do, so it earns no mark —
//! marking it would be one more icon on every card in the workbench saying that
//! nothing has happened.
//!
//! Which leaves three states drawn the same way, and that is deliberate: GitHub
//! saying it merges, GitHub still working the answer out, and nothing ever
//! having asked are all a card with no mark on it. Only the conflict is written
//! down as news, in the same spirit the check rollup is never guessed at — see
//! `Checks.tsx`, whose reasoning this is the second half of.
//!
//! `fa-code-merge` because the mark is about merging rather than about failing:
//! a cross beside the rollup's own cross would read as a second red check, and
//! this is not a check. The colour is what says it will not merge, and the shape
//! is what says what it is about.
//!
//! The card's mark is the whole of what a screen reader gets from it, the icon
//! carrying no text — so the words are here, the way the rollup's are.

import { faCodeMerge } from "@fortawesome/free-solid-svg-icons";
import { Show, type JSX } from "solid-js";

import { Icon } from "../Icon";
import type { Merging } from "../api/types";
import styles from "./Merging.module.css";

/// What the mark says when it is read aloud, which is what the icon shows said
/// in words.
export const CONFLICTED = "it conflicts with its base";

/// And what the details pane says about it, which is the same fact with room to
/// say what follows from it.
export const IN_WORDS =
  "This conflicts with its base, so nothing lands until the conflict is resolved.";

/// The mark for one pull request, or nothing where there is nothing to mark.
///
/// Nothing is the answer for both the other readings and for the absence of one:
/// see the module docs, where that is argued.
export function Conflict(props: {
  merging: Merging | null;
  class?: string;
}): JSX.Element {
  return (
    <Show when={props.merging === "Conflicting"}>
      <Icon
        of={faCodeMerge}
        label={CONFLICTED}
        class={[styles.conflict, props.class].filter(Boolean).join(" ")}
      />
    </Show>
  );
}
