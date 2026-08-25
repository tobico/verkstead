//! The small act, drawn as a quiet outline: the button on a section's heading
//! that opens its form, and the two on a profile's row that change one or take
//! it away.
//!
//! One control repeated four times across three pages, and it was one rule with
//! four selectors while there was a single stylesheet to write it in. Splitting
//! that sheet into modules split the rule with it — three of the four ended up
//! spelling the same eight declarations out again in the module of whatever
//! page they were on — so what was one thing to change became four.
//!
//! A component rather than a class handed round, because a class handed round
//! is the same duplication with an import in front of it: what these four share
//! is the control, not a name for its paint. A caller that wants one drawn a
//! little differently — the remove on a profile row, which is the one act here
//! that takes something away — styles the class it hands down, the way the
//! menu, the modal and the notices already work.

import type { JSX } from "solid-js";

import styles from "./QuietButton.module.css";

/// One small act on whatever it stands beside.
export function QuietButton(props: {
  /// What pressing it does.
  onClick: () => void;
  /// A class of the caller's, for the one that has to read differently from the
  /// rest. Styled by whoever passes it, never here.
  class?: string;
  /// Its words, which are the whole of what it is called.
  children: JSX.Element;
}): JSX.Element {
  return (
    <button
      type="button"
      class={[styles.quiet, props.class].filter(Boolean).join(" ")}
      onClick={() => props.onClick()}
    >
      {props.children}
    </button>
  );
}
