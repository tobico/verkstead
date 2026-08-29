//! A timeline document, opened: the whole of its markdown in the details pane.
//!
//! Three events are a document and nothing else — the handoff the grilling
//! wrote, what a steer sent a session off with (an instruction, or the brief a
//! follow-up was opened on), and a Notice Verkstead wrote on its own account.
//! Each card is cut down so that a long one stops pushing the record down —
//! the two documents to five lines under a fade, a notice to one line under an
//! ellipsis — which makes the rest of any of them something to open; and what
//! is opened is the same markdown uncut, so one component draws all three.
//!
//! The frozen Brief is the third card of that shape and is drawn by `Brief.tsx`
//! instead: its pane carries the summary of what the Conversation was configured
//! with under the markdown, which neither of these two has anything like.
//!
//! It is carried by the Conversation rather than fetched, unlike a Capture or a
//! diff: the timeline already has every byte of it — the card was drawn from the
//! same HTML — so a fetch here would ask the server for what this page is
//! holding.

import { Show, type JSX } from "solid-js";

import { Empty } from "../notices";
import styles from "./Document.module.css";
import { PaneHead } from "./PaneHead";

export function Document(props: {
  /// What the pane is titled, which is what the card is titled.
  heading: string;
  /// The markdown, rendered and sanitized by the server on the way out.
  html: string;
  /// What it says where the document is empty, in the human's terms rather than
  /// as a blank pane.
  empty: string;
  back: () => void;
}): JSX.Element {
  return (
    <>
      <PaneHead back={{ to: "Timeline", go: props.back }} title={props.heading} />

      <Show when={props.html !== ""} fallback={<Empty>{props.empty}</Empty>}>
        <div class={`${styles.document} markdown`} innerHTML={props.html} />
      </Show>
    </>
  );
}
