//! The mark a session's liveness is said in: a turning ring, or a still one.
//!
//! The same mark the sidebar's conversation card carries, drawn here for the
//! Timeline row and the details pane so that a session's liveness reads the same
//! wherever it is said. A word saying `running` said it once and said nothing
//! about a session that had stopped talking an hour ago — and a Timeline of
//! rows is exactly where a badge shouting is worse than a mark noticed out of
//! the corner of an eye.
//!
//! Three states from two flags. A session that is not running has no mark at
//! all, because there is nothing happening for one to be about; a running one
//! turns; and a running one that has gone quiet is the same ring held still,
//! which is what an empty circle is. The empty circle is deliberately the
//! *lesser* mark: what it means is that the session is sitting there, which is
//! nothing to look at until the human decides otherwise.
//!
//! Drawn rather than written, so it carries its own label: everywhere else the
//! mark is used it is a button's `aria-label` that says what it means, and a
//! Timeline row's label is the row.

import { Show, type JSX } from "solid-js";

/// What each mark says when it is read aloud, which is the whole of what a
/// screen reader gets from one — see the module note above.
///
/// Exported for the sidebar's conversation card, which draws its own mark
/// inside an already-labelled button and so needs the words rather than the
/// element: a ring means the same thing wherever it is drawn, and it should
/// mean it in the same words too.
export const SPOKEN = {
  working: "a session is running",
  idle: "a session is running and has gone quiet",
} as const;

/// The mark for one session, or nothing where it is over.
export function Mark(props: { running: boolean; idle: boolean }): JSX.Element {
  const which = () => (props.idle ? "idle" : "working");

  return (
    <Show when={props.running}>
      <span class={`mark ${which()}`} role="img" aria-label={SPOKEN[which()]} />
    </Show>
  );
}
