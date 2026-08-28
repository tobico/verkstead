//! The mark a pull request's checks are said in: passed, still running, failed.
//!
//! GitHub's own three, drawn rather than worded for the reason the liveness ring
//! is — a card in a list is glanced at, and a card saying `checks: passing` in
//! words is one more line to read on a Timeline of them. The shapes are the ones
//! GitHub puts beside a pull request, because this is a reading of GitHub and
//! the human has just come from looking at it there: a tick for a green suite, a
//! cross for a red one, and a dot for a suite that has not finished.
//!
//! One icon for a whole suite, which is what a card has room for: which of the
//! three it is, and not what each check in it is called.
//!
//! Nothing at all where nothing is known — a pull request in a repository with
//! no CI, one nothing has asked about yet, and one opened before Verkstead wrote
//! this down. *Not known* is a third thing beside green and red, and an icon
//! guessing at it would be worse than no icon.

import { Show, type JSX } from "solid-js";

import type { CheckRollup } from "../api/types";
import styles from "./Checks.module.css";

/// What each mark says when it is read aloud, which is the whole of what a
/// screen reader gets from one — the icon carries no text.
export const SPOKEN: Record<CheckRollup, string> = {
  Passed: "the checks passed",
  Running: "the checks are still running",
  Failed: "a check failed",
};

/// Which rule draws each of them.
const DRAWN: Record<CheckRollup, string> = {
  Passed: styles.passed!,
  Running: styles.running!,
  Failed: styles.failed!,
};

/// The mark for one pull request's checks, or nothing where there is none.
export function Checks(props: {
  checks: CheckRollup | null;

  /// Where the mark stands in the line it was drawn into — the `class` prop the
  /// liveness ring beside it takes. Which icon is drawn is this component's;
  /// where it sits belongs to whatever drew the line.
  class?: string;
}): JSX.Element {
  return (
    <Show when={props.checks}>
      {(rollup) => (
        <span
          class={[styles.checks, DRAWN[rollup()], props.class]
            .filter(Boolean)
            .join(" ")}
          role="img"
          aria-label={SPOKEN[rollup()]}
        />
      )}
    </Show>
  );
}
