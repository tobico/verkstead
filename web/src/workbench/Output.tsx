//! One session's output, opened: the whole transcript, in the details pane.
//!
//! Fetched here rather than carried by the Conversation, because the two are
//! different sizes. A session prints for an hour and the Timeline is read again
//! every time the page hears the world moved; the transcript is read when
//! somebody opens the one Event it belongs to — and then it is read again on
//! every Nudge, which is what makes an open transcript follow a running session.
//!
//! Shown as it was printed, control sequences and all. What a terminal was sent
//! is what the session said, and turning a terminal's own colours into markup is
//! rendering — which on this wire is the server's, in the crate that has the
//! parsers in it, rather than something a browser is handed a second
//! implementation of.

import { useQuery } from "@tanstack/solid-query";
import { Match, Show, Switch, type JSX } from "solid-js";

import { loadTranscript } from "../api/client";
import type { AgentOutputEvent, ConversationView } from "../api/types";

export function Output(props: {
  conversation: ConversationView;
  output: AgentOutputEvent;
  back: () => void;
  close: () => void;
}): JSX.Element {
  const transcript = useQuery(() => ({
    // The Event is in the key, so opening another session's output is another
    // query rather than the same one showing the wrong transcript for a moment.
    queryKey: ["transcript", props.conversation.id, props.output.id],
    queryFn: () => loadTranscript(props.conversation.id, props.output.id),
  }));

  return (
    <>
      <div class="pane-head">
        <button type="button" class="pane-back" onClick={props.back}>
          ← Timeline
        </button>
        <h1>Agent output</h1>
        {/* The way back to what the conversation is, which is what this pane
            shows when no event is open. */}
        <button type="button" class="close-event" onClick={props.close}>
          Close
        </button>
      </div>

      <p class="transcript-summary">
        {props.output.lines} {props.output.lines === 1 ? "line" : "lines"}
        <Show when={props.output.running}>
          <span class="live">running</span>
        </Show>
      </p>

      <Switch>
        <Match when={transcript.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={transcript.isError}>
          <p class="error">
            Could not read this transcript: {transcript.error?.message}
          </p>
        </Match>
        <Match when={transcript.data}>
          {(transcript) => (
            <Show
              when={transcript().text !== ""}
              fallback={
                <p class="empty">This session has printed nothing yet.</p>
              }
            >
              <pre class="transcript">{transcript().text}</pre>
            </Show>
          )}
        </Match>
      </Switch>
    </>
  );
}
