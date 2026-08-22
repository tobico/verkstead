//! One session, opened: what it said, in the details pane.
//!
//! Two records of the same session, and the pane shows whichever there is. The
//! Transcript is the session's own account of its conversation — the agent's
//! prose, its reasoning, the tools it called and the turns put to it — and the
//! Capture is the bytes it sent a terminal. Where a session left a Transcript
//! that is what a reader came for; where it left none, which is every stub agent
//! and every backend that keeps no such log, the Capture is a complete record on
//! its own and is drawn exactly as it always was.
//!
//! Fetched here rather than carried by the Conversation, because of the two
//! sizes involved. A session talks for an hour and the Timeline is read again
//! every time the page hears the world moved; either record is read when
//! somebody opens the one Event it belongs to — and then again on every Nudge,
//! which is what makes an open pane follow a running session.
//!
//! Nothing here parses anything. The lines a backend wrote are read on the
//! server, in the crate that has the parsers in it, and what arrives is the
//! turns already rendered — so no markdown parser and no reader of somebody
//! else's file format ships to the browser. The Capture is the same rule from
//! the other end: turning a terminal's own colours into markup would be
//! rendering too, so it is shown as it was printed, control sequences and all.
//!
//! Reasoning and tool calls arrive collapsed, and prose does not. What a reader
//! opened this for is what the agent said; what it was thinking and what it ran
//! are there to be opened, one at a time, rather than to be scrolled past. The
//! backend's own bookkeeping — modes, reminders, attachments, snapshots — is a
//! third of every log and none of what anybody came for, so it folds into one
//! group at the end: nothing hidden, and nothing in the way.

import { useQuery } from "@tanstack/solid-query";
import { For, Match, Show, Switch, type JSX } from "solid-js";

import { loadCapture, loadTranscript } from "../api/client";
import type {
  AgentOutputEvent,
  Bookkeeping,
  ConversationView,
  Turn,
} from "../api/types";

export function Output(props: {
  conversation: ConversationView;
  output: AgentOutputEvent;
  back: () => void;
  close: () => void;
}): JSX.Element {
  const transcript = useQuery(() => ({
    // The Event is in the key, so opening another session's output is another
    // query rather than the same one showing the wrong session for a moment.
    queryKey: ["transcript", props.conversation.id, props.output.id],
    queryFn: () => loadTranscript(props.conversation.id, props.output.id),
  }));

  /// Whether this session left a record of its own conversation.
  ///
  /// Asked of what came back rather than of the Event, because nothing on the
  /// Timeline knows: a Transcript with nothing on it is a session that kept no
  /// log, a backend that keeps none, and a session that has not said anything
  /// yet, and all three are read back off the Capture.
  const spoke = () => {
    const read = transcript.data;
    return (
      read !== undefined &&
      (read.turns.length > 0 || read.bookkeeping.length > 0)
    );
  };

  const capture = useQuery(() => ({
    queryKey: ["capture", props.conversation.id, props.output.id],
    queryFn: () => loadCapture(props.conversation.id, props.output.id),
    // Only for the session that left no Transcript. A second request every time
    // a pane is opened would be a request for something nobody is going to read.
    enabled: transcript.data !== undefined && !spoke(),
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

      <p class="capture-summary">
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
            Could not read what this session said: {transcript.error?.message}
          </p>
        </Match>
        <Match when={spoke() && transcript.data}>
          {(said) => (
            <>
              <ol class="transcript">
                <For each={said().turns}>
                  {(turn) => <Said turn={turn} />}
                </For>
              </ol>
              <Kept lines={said().bookkeeping} />
            </>
          )}
        </Match>
        {/* No Transcript, so the bytes — which is the whole details-pane story
            for a session whose backend kept no log of itself. */}
        <Match when={capture.isError}>
          <p class="error">
            Could not read this capture: {capture.error?.message}
          </p>
        </Match>
        <Match when={capture.data}>
          {(capture) => (
            <Show
              when={capture().text !== ""}
              fallback={
                <p class="empty">This session has printed nothing yet.</p>
              }
            >
              <pre class="capture">{capture().text}</pre>
            </Show>
          )}
        </Match>
        <Match when={true}>
          <p class="empty">Loading…</p>
        </Match>
      </Switch>
    </>
  );
}

/// One turn, drawn as whichever of the six it is.
///
/// Each is its own element with its own class, because the whole of what makes a
/// Transcript readable is that a reader can tell a person's turn from a tool's
/// answer without reading either — and the two arrive from the log under the
/// same type, which is why the server told them apart before this got them.
function Said(props: { turn: Turn }): JSX.Element {
  return (
    <Switch>
      <Match when={"Prose" in props.turn && props.turn.Prose}>
        {(prose) => (
          <li class="turn prose">
            <div class="markdown" innerHTML={prose().html} />
          </li>
        )}
      </Match>

      <Match when={"Reasoning" in props.turn && props.turn.Reasoning}>
        {(reasoning) => (
          <li class="turn reasoning">
            <details>
              <summary>Thinking</summary>
              <div class="markdown" innerHTML={reasoning().html} />
            </details>
          </li>
        )}
      </Match>

      <Match when={"ToolUse" in props.turn && props.turn.ToolUse}>
        {(call) => (
          <li class="turn tool-use">
            <details>
              <summary>
                <span class="tool">{call().name}</span>
                <Show when={call().about}>
                  <span class="about">{call().about}</span>
                </Show>
              </summary>
              <pre class="input">{call().input}</pre>
            </details>
          </li>
        )}
      </Match>

      <Match when={"ToolResult" in props.turn && props.turn.ToolResult}>
        {(answer) => (
          <li class="turn tool-result" classList={{ failed: answer().failed }}>
            <details>
              <summary>{answer().failed ? "Failed" : "Result"}</summary>
              <pre class="output">{answer().text}</pre>
            </details>
          </li>
        )}
      </Match>

      <Match when={"Put" in props.turn && props.turn.Put}>
        {(put) => (
          <li class="turn put">
            <div class="markdown" innerHTML={put().html} />
          </li>
        )}
      </Match>

      {/* A line of a kind this version has never met, shown as the JSON it is
          rather than dropped: a format that has moved on should say so here
          instead of quietly emptying the pane. */}
      <Match when={"Unread" in props.turn && props.turn.Unread}>
        {(unread) => (
          <li class="turn unread">
            <details>
              <summary>A line this version does not know</summary>
              <pre class="raw">{unread().line}</pre>
            </details>
          </li>
        )}
      </Match>
    </Switch>
  );
}

/// The backend's own bookkeeping, in one group at the end.
///
/// One group for the whole session rather than one row where each line fell:
/// these are not turns and putting them in the order they arrived would be
/// putting them back in the way, which is the thing folding them away is for.
function Kept(props: { lines: Bookkeeping[] }): JSX.Element {
  return (
    <Show when={props.lines.length > 0}>
      <details class="bookkeeping">
        <summary>
          {props.lines.length}{" "}
          {props.lines.length === 1 ? "bookkeeping line" : "bookkeeping lines"}
        </summary>
        <ol>
          <For each={props.lines}>
            {(kept) => (
              <li>
                <span class="kind">{kept.kind}</span>
                <pre class="raw">{kept.line}</pre>
              </li>
            )}
          </For>
        </ol>
      </details>
    </Show>
  );
}
