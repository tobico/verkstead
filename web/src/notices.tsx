//! The three lines the app says when it has nothing better to show: that there
//! is nothing here, that something went wrong, and what a control does not say
//! for itself.
//!
//! There were three classes and no components — `.empty` across sixteen
//! components, `.error` across seventeen, `.note` across seven — each styled
//! once and then refined by a dozen parents that wanted it a little tighter or
//! a little quieter. The vocabulary was real and worth keeping; what was not
//! worth keeping was that all hundred-odd sites spelled it out as a string, so
//! nothing could be renamed and nothing could be found.
//!
//! So the vocabulary becomes three components over one module, and the string
//! becomes an import. The refinements stay where they were — with the parent —
//! through the `class` prop the menu and the modal already take: a parent that
//! wants this line indented or in its own colour styles the class it hands
//! down, and what it writes lands beside the base rather than instead of it.

import type { JSX } from "solid-js";
import { Dynamic } from "solid-js/web";

import styles from "./notices.module.css";

/// What all three are told.
type Notice = {
  /// A class of the parent's, for the context that wants this line drawn a
  /// little differently from every other one. Styled by whoever passes it,
  /// never here.
  class?: string;
  /// The words.
  children: JSX.Element;
};

/// Whether a line is a paragraph of its own or a word in somebody else's.
type Placed = Notice & {
  /// Drawn as a `span` rather than a `p`, for the two places one of these
  /// sits inside a line that is already running — the provenance line a Set's
  /// standing is on, and the last output beside an agent's heading. A `p`
  /// there would break the line it was put in.
  inline?: boolean;
};

/// Nothing to show — an empty list, or one still being read.
export function Empty(props: Placed): JSX.Element {
  return <Line of={styles.empty} notice={props} />;
}

/// What went wrong: a refusal the server named, or a request that never landed.
///
/// `ErrorLine` rather than `Error`, because the other `Error` is the one every
/// failed mutation carries and a file holding both would read as a bug.
export function ErrorLine(props: Placed): JSX.Element {
  return <Line of={styles.error} notice={props} />;
}

/// What a control does not say for itself, said underneath it in words.
export function Note(props: Notice): JSX.Element {
  return <Line of={styles.note} notice={props} />;
}

/// And the rest of what a note would say, folded behind a line naming it: what
/// the copy guide means by a disclosure. A note beside a control is a sentence
/// or two, and what somebody needs only when they go looking — the rules a
/// field's value follows, how a mechanism works — waits here rather than being
/// read past every time the pane is opened.
export function More(props: Notice & { summary: string }): JSX.Element {
  return (
    <details class={[styles.more, props.class].filter(Boolean).join(" ")}>
      <summary>{props.summary}</summary>
      <div>{props.children}</div>
    </details>
  );
}

/// The one element all three are, told which of the module's classes says
/// which.
///
/// The class arrives as `string | undefined` because every lookup in a CSS
/// module's object does: the sheet is the single spelling of these names, so
/// there is nothing for the compiler to check one against. An undefined one is
/// dropped rather than written into the attribute as the word itself.
function Line(props: { of: string | undefined; notice: Placed }): JSX.Element {
  return (
    <Dynamic
      component={props.notice.inline ? "span" : "p"}
      class={[props.of, props.notice.class].filter(Boolean).join(" ")}
    >
      {props.notice.children}
    </Dynamic>
  );
}
