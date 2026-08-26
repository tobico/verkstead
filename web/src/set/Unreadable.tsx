//! A Question Set this build cannot read, drawn as what there is of it.
//!
//! The record and not the document. A stored body the schema will not take any
//! more — a field retired, and nothing else wrong with it — is still the record
//! of what an agent asked, so it is kept and shown as the JSON it is rather than
//! hidden behind a failed page. That is ADR 0006's rule for a Transcript line
//! nothing knows how to draw, applied to the Sets themselves.
//!
//! Nothing to press. Answering is checked against Questions nobody here can
//! read, so there is no sheet to fill in and no Response to send; archiving is
//! not offered either, because closing a Set unanswered is a thing done about a
//! Set somebody is reading, and this is a record to be fixed by a build that can
//! read it.
//!
//! Its own module rather than the page's, for [`Sheet`](./Sheet.tsx)'s reason: a
//! Set is reached two ways — as a page of its own and as the details pane of the
//! Timeline Event it landed on — and one drawing between them is one reading.

import type { JSX } from "solid-js";

import app from "../App.module.css";
import type { UnreadableSet } from "../api/types";
// The provenance line is the sheet's: a Set this build cannot read still says
// where it came from, in the same place a readable one does.
import page from "./Sheet.module.css";
import styles from "./Unreadable.module.css";

/// One unreadable Set, top to bottom: what it is, why this build cannot read it,
/// and the body as it was stored.
///
/// `lead` is whatever the record is reached through — the way back to the
/// Conversation it was asked from, or the pane header of the Timeline Event it
/// belongs to — exactly as it is on a readable Set's sheet.
export function Unreadable(props: {
  set: UnreadableSet;
  lead?: JSX.Element;
}): JSX.Element {
  return (
    <>
      {props.lead}
      <h1>Question set {props.set.id}</h1>
      <div class={page.meta}>
        <span class={styles.unreadableBadge}>cannot be read</span>
      </div>
      <p class={styles.unreadableWhy}>
        This build cannot read the Set as it was stored, so there is nothing to
        draw the questions from and nothing to answer. What was asked is still on
        the record, below, exactly as it was written: {props.set.why}
      </p>
      {/* The body verbatim, because verbatim is the whole of what is left. It
          scrolls inside its own block rather than widening the page — a stored
          Set carries a Preface and a Diff, and one long line of JSON is not a
          reason for the column beside it to move. */}
      <section class={styles.storedBody} id="stored-body">
        <h2 class={app.sectionHeading}>As it was stored</h2>
        <pre class={styles.storedJson}>{props.set.body}</pre>
      </section>
    </>
  );
}
