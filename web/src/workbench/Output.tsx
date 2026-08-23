//! One session, opened: what it said and how it looked, in the details pane.
//!
//! A two-way switch in the pane's header, opening on the Transcript because
//! that is what a reader usually came for. The Transcript is the session's own
//! account of its conversation — the agent's prose, its reasoning, the tools it
//! called and the turns put to it — and where a session left none, which is
//! every stub agent and every backend that keeps no such log, the Capture stands
//! in its place and is drawn exactly as it always was.
//!
//! The Screen is the same session read the other way: not the bytes it sent a
//! terminal but the terminal at the other end of them, drawn as a terminal — see
//! [`./Screen`], which is where the one exception to the rule below lives.
//!
//! Fetched here rather than carried by the Conversation, because of the two
//! sizes involved. A session talks for an hour and the Timeline is read again
//! every time the page hears the world moved; either record is read when
//! somebody opens the one Event it belongs to — and then again on every Nudge,
//! which is what makes an open pane follow a running session.
//!
//! And the Transcript is read on from where it got to rather than from the
//! start, because that following is twice a second while a session talks: what
//! the pane holds is the cursor its last reading ended at, what arrives is what
//! has been said since, and the two are added together here (ADR-0009). The
//! Capture is not — it stands in only for the sessions that kept no log, and
//! incremental reads stop at the Transcript until that pane hurts.
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

import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  type JSX,
} from "solid-js";

import { loadCapture, loadTranscript } from "../api/client";
import { useReading } from "../freshness";
import { Mark } from "./Mark";
import { Screen } from "./Screen";
import type {
  AgentOutputEvent,
  Bookkeeping,
  ConversationView,
  TranscriptView,
  Turn,
} from "../api/types";

export function Output(props: {
  conversation: ConversationView;
  output: AgentOutputEvent;
  back: () => void;
}): JSX.Element {
  // Whether this session was already over when the pane opened, read once as
  // the pane is set up rather than tracked: a session over before it was
  // opened is over for good, and one that was running stays live for as long
  // as the pane is. Following the flip live would race the very Nudge carrying
  // the session's last words, and losing that race would leave a Transcript
  // quietly missing its ending.
  const over = !props.output.running;

  /// The record as far as this pane has read it, and whose it is.
  ///
  /// Kept beside the query rather than read back out of it, because what the
  /// next read needs is the cursor the last one ended at — and the query holds
  /// the drawn record, which reconcile owns and rewrites in place. Which
  /// session's it is, because a pane is pointed at another output without being
  /// built again, and a cursor belongs to the record it was read from.
  let read: { of: number; record: TranscriptView } | undefined;

  const transcript = useReading(() => ({
    // The Event is in the key, so opening another session's output is another
    // query rather than the same one showing the wrong session for a moment.
    queryKey: ["transcript", props.conversation.id, props.output.id],

    // Only what the session has said since this pane last looked, which while
    // it is talking is a line or two against an hour of them (ADR 0009). The
    // first read of a session asks for no such thing and gets the record; so
    // does every read the server cannot carry on from, which is why it is the
    // arriving payload that says whether to add or to replace.
    queryFn: async () => {
      const before = read?.of === props.output.id ? read.record : undefined;
      const arrived = await loadTranscript(
        props.conversation.id,
        props.output.id,
        before?.cursor,
      );

      const record =
        before && !arrived.whole
          ? {
              turns: [...before.turns, ...arrived.turns],
              bookkeeping: [...before.bookkeeping, ...arrived.bookkeeping],
              // What the accumulation is: the record from its beginning.
              whole: true,
              cursor: arrived.cursor,
            }
          : arrived;

      read = { of: props.output.id, record };
      return record;
    },

    // A finished session's record cannot change, so it is read once and never
    // again. While the session is still talking it is re-read on every Nudge,
    // and each read is merged into the turns already drawn rather than
    // replacing them, as the Workbench does its Timeline: without the merge
    // every turn comes back as a new object, `For` rebuilds every row, and any
    // fold the reader had opened snaps shut with the element it was DOM state
    // on. Keyed by `id`, which is the turn's place in the conversation — and
    // flat on the turn itself, because reconcile does not look inside. Which is
    // also what makes the accumulation above cost nothing: the turns it carries
    // over are the turns already drawn, so the merge leaves every one of them
    // and its element alone.
    freshness: over ? "static" : { reconcile: "id" },
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

  const capture = useReading(() => ({
    queryKey: ["capture", props.conversation.id, props.output.id],
    queryFn: () => loadCapture(props.conversation.id, props.output.id),
    // Only for the session that left no Transcript. A second request every time
    // a pane is opened would be a request for something nobody is going to read.
    enabled: transcript.data !== undefined && !spoke(),

    // Frozen with the Transcript, for the same reason — and merged while the
    // session runs, for a lesser one: what a running stub session prints is
    // re-read whole on every Nudge, and the merge leaves the one field of it
    // alone on the reads that added nothing.
    freshness: over ? "static" : { reconcile: "id" },
  }));

  /// Which of the two records is showing.
  ///
  /// The Transcript to begin with, because it is what a reader usually came for:
  /// what the session *said*. The Screen is how it looked while it said it, and
  /// it is a click away rather than a scroll away.
  ///
  /// Except on the session whose keyboard the human has taken, which opens on
  /// the Screen. That is what the *blocked on you* badge points at and what
  /// handing back is pressed on — a Hold that opened behind the other tab would
  /// be one the human had to go looking for.
  const [showing, setShowing] = createSignal<"transcript" | "screen">(
    props.conversation.held === props.output.id ? "screen" : "transcript",
  );

  /// The two labels themselves, so the mark under the pressed one can be put
  /// where that one is.
  let transcriptTab!: HTMLButtonElement;
  let screenTab!: HTMLButtonElement;

  /// Where that mark stands: the pixels the pressed label occupies.
  ///
  /// Measured rather than written down, because the two labels are words of
  /// different lengths and the mark travels between them. A rule could put a
  /// background on whichever button is pressed, but a background cannot move
  /// from one element to another, and moving is the whole of what this is for.
  ///
  /// Read off `offsetLeft` and `offsetWidth`, which are the switch's own
  /// coordinates — it is the offset parent and the containing block both — so
  /// where the header has put it makes no difference and a header that wraps
  /// needs no measuring again. What these do follow is the font, and that is
  /// settled long before anybody presses anything.
  const [mark, setMark] = createSignal({ at: 0, wide: 0 });

  createEffect(() => {
    const pressed = showing() === "transcript" ? transcriptTab : screenTab;
    setMark({ at: pressed.offsetLeft, wide: pressed.offsetWidth });
  });

  return (
    <>
      <div class="pane-head">
        <button type="button" class="pane-back" onClick={props.back}>
          ← Timeline
        </button>
        <h1>Agent output</h1>

        {/* The two ways of reading the one session, beside the title rather than
            across the pane under it: two words is all the width it ever needs,
            and the header is where a pane's own controls belong. Buttons that
            say which they are rather than tabs — there are two of them, both are
            always there, and `aria-pressed` is the one word that says which is
            showing.

            The mark under the pressed one is presentation and nothing else, so
            it is hidden from anybody being read to: what it draws is what
            `aria-pressed` has already said, and it exists so that switching
            reads as one thing moving rather than two things blinking. */}
        <div
          class="record-switch"
          role="group"
          aria-label="How to read this session"
        >
          <span
            class="indicator"
            aria-hidden="true"
            style={{
              transform: `translateX(${mark().at}px)`,
              width: `${mark().wide}px`,
            }}
          />
          <button
            type="button"
            class="transcript-tab"
            ref={transcriptTab}
            aria-pressed={showing() === "transcript"}
            onClick={() => setShowing("transcript")}
          >
            Transcript
          </button>
          <button
            type="button"
            class="screen-tab"
            ref={screenTab}
            aria-pressed={showing() === "screen"}
            onClick={() => setShowing("screen")}
          >
            Screen
          </button>
        </div>
      </div>

      {/* The same metric the Timeline row shows, and absent for the same
          reason: a session with no Transcript has no turns to count. A finished
          session with none has nothing to say here at all, so the line itself
          goes rather than standing empty above the record. */}
      <Show when={props.output.turns !== null || props.output.running}>
        <p class="capture-summary">
          <Show when={props.output.turns !== null}>
            <span class="turns">
              {props.output.turns} {props.output.turns === 1 ? "turn" : "turns"}
            </span>
          </Show>
          {/* And the same mark the row this was opened from carries: one
              session's liveness, said the one way. */}
          <Mark running={props.output.running} idle={props.output.idle} />
        </p>
      </Show>

      <Show
        when={showing() === "transcript"}
        fallback={
          <Screen conversation={props.conversation} output={props.output} />
        }
      >
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
          {/* No Transcript, so the bytes — which is the whole Transcript-side
              story for a session whose backend kept no log of itself. */}
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
      </Show>
    </>
  );
}

/// One turn, drawn as whichever of the six its `kind` says it is.
///
/// Each is its own element with its own class, because the whole of what makes a
/// Transcript readable is that a reader can tell a person's turn from a tool's
/// answer without reading either — and the two arrive from the log under the
/// same type, which is why the server told them apart before this got them.
function Said(props: { turn: Turn }): JSX.Element {
  return (
    <Switch>
      <Match when={props.turn.kind === "Prose" && props.turn}>
        {(prose) => (
          <li class="turn prose">
            <div class="markdown" innerHTML={prose().html} />
          </li>
        )}
      </Match>

      <Match when={props.turn.kind === "Reasoning" && props.turn}>
        {(reasoning) => (
          <li class="turn reasoning">
            <details>
              <summary>Thinking</summary>
              <div class="markdown" innerHTML={reasoning().html} />
            </details>
          </li>
        )}
      </Match>

      <Match when={props.turn.kind === "ToolUse" && props.turn}>
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

      <Match when={props.turn.kind === "ToolResult" && props.turn}>
        {(answer) => (
          <li class="turn tool-result" classList={{ failed: answer().failed }}>
            <details>
              <summary>{answer().failed ? "Failed" : "Result"}</summary>
              <pre class="output">{answer().text}</pre>
            </details>
          </li>
        )}
      </Match>

      <Match when={props.turn.kind === "Put" && props.turn}>
        {(put) => (
          <li class="turn put">
            <div class="markdown" innerHTML={put().html} />
          </li>
        )}
      </Match>

      {/* A line of a kind this version has never met, shown as the JSON it is
          rather than dropped: a format that has moved on should say so here
          instead of quietly emptying the pane. */}
      <Match when={props.turn.kind === "Unread" && props.turn}>
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
