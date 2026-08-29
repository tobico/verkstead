//! The mark a pull request's checks are said in: passed, still running, failed.
//!
//! GitHub's own three, drawn rather than worded for the reason the liveness ring
//! is — a card in a list is glanced at, and a card saying `checks: passing` in
//! words is one more line to read on a Timeline of them. The shapes are the ones
//! GitHub puts beside a pull request, because this is a reading of GitHub and
//! the human has just come from looking at it there: a tick for a green suite, a
//! cross for a red one, and a ring for a suite that has not finished.
//!
//! Font Awesome's three rather than three shapes built here out of borders and
//! rotations, which is what they were: a tick was the right and bottom edges of
//! a box turned onto its corner, and nobody could change its shape without
//! doing the arithmetic again. What each of the three *means* is still this
//! file's, and so is the colour — the icons are ink and the palette says which
//! ink. See `Icon.tsx` for how one is drawn.
//!
//! One icon for a whole suite, which is what a card has room for: which of the
//! three it is, and not what each check in it is called. The details pane draws
//! the same three shapes beside each check by name — one mark, so a suite and
//! the checks inside it are read in one alphabet.
//!
//! Nothing at all where nothing is known — a pull request in a repository with
//! no CI, one nothing has asked about yet, and one opened before Verkstead wrote
//! this down. *Not known* is a third thing beside green and red, and an icon
//! guessing at it would be worse than no icon.

import { faCircle } from "@fortawesome/free-regular-svg-icons";
import type { IconDefinition } from "@fortawesome/free-solid-svg-icons";
import { faCheck, faXmark } from "@fortawesome/free-solid-svg-icons";
import { Show, type JSX } from "solid-js";

import { Icon } from "../Icon";
import type { CheckRollup, Checked } from "../api/types";
import styles from "./Checks.module.css";

/// The three words either of them is said in, which are the same three: a whole
/// suite and one check of it are drawn the same way and differ in what they are
/// about.
type Mark = CheckRollup | Checked;

/// What a suite's mark says when it is read aloud, which is the whole of what a
/// screen reader gets from one — the icon carries no text.
export const SPOKEN: Record<CheckRollup, string> = {
  Passed: "the checks passed",
  Running: "the checks are still running",
  Failed: "a check failed",
};

/// And what one check's says, which is about the check its name sits beside
/// rather than about the suite.
export const SAID: Record<Checked, string> = {
  Passed: "passed",
  Running: "still running",
  Failed: "failed",
};

/// Which shape each of them is drawn as.
///
/// The empty ring for a suite that has not finished, which is the one of the
/// three that says nothing has happened yet: a tick and a cross are both an
/// outcome, and the ring is the shape they are cut into once there is one.
const SHAPE: Record<Mark, IconDefinition> = {
  Passed: faCheck,
  Running: faCircle,
  Failed: faXmark,
};

/// And which rule colours it.
const DRAWN: Record<Mark, string> = {
  Passed: styles.passed!,
  Running: styles.running!,
  Failed: styles.failed!,
};

/// One of the three shapes, with the words for whoever cannot see it.
///
/// The drawing on its own, so the card's rollup and the pane's per-check line
/// are the one mark: what differs between them is what the mark is about, which
/// is what it is read aloud as.
export function CheckMark(props: {
  how: Mark;

  /// What it says when it is read aloud.
  spoken: string;

  /// Where the mark stands in the line it was drawn into — the `class` prop the
  /// liveness ring beside it takes. Which icon is drawn is this component's;
  /// where it sits belongs to whatever drew the line.
  class?: string;
}): JSX.Element {
  return (
    <Icon
      of={SHAPE[props.how]}
      label={props.spoken}
      class={[styles.checks, DRAWN[props.how], props.class]
        .filter(Boolean)
        .join(" ")}
    />
  );
}

/// The mark for one pull request's checks, or nothing where there is none.
export function Checks(props: {
  checks: CheckRollup | null;
  class?: string;
}): JSX.Element {
  return (
    <Show when={props.checks}>
      {(rollup) => (
        <CheckMark
          how={rollup()}
          spoken={SPOKEN[rollup()]}
          class={props.class}
        />
      )}
    </Show>
  );
}
