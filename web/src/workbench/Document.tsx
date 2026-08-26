//! A timeline document, opened: the whole of its markdown in the details pane.
//!
//! Three events are documents and nothing else — the frozen Brief, the handoff
//! the grilling wrote, and the instruction a steer sent a session off with.
//! Their cards clamp to five lines so that a long one stops pushing the record
//! down, which makes the rest of them something to open; and what is opened is
//! the same markdown without the clamp, so one component draws all three.
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
  close: () => void;
}): JSX.Element {
  return (
    <>
      {/* The Close is the way back to what the conversation is, which is what
          this pane shows when no event is open. */}
      <PaneHead
        back={{ to: "Timeline", go: props.back }}
        title={props.heading}
        close={props.close}
      />

      <Show when={props.html !== ""} fallback={<Empty>{props.empty}</Empty>}>
        <div class={`${styles.document} markdown`} innerHTML={props.html} />
      </Show>
    </>
  );
}
