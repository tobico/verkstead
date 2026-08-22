//! One session's Screen: the grid its Capture leaves on a terminal, drawn as a
//! terminal.
//!
//! The window onto the server's own — and the one deliberate exception to the
//! rule that the browser never parses, argued in [ADR
//! 0007](../../../docs/adr/0007-server-held-terminal.md). A live terminal is a
//! terminal rather than a document: shipping a rendered grid down the wire would
//! cost latency and bytes for no fidelity at all, and the escape sequences are
//! already the shorter description. What keeps the exception bounded is that the
//! server's virtual terminal stays the source of truth — this one is handed a
//! repaint and paints it, and never decides anything.
//!
//! Read-only, and in two ways at once. The terminal takes no input, because
//! there is nowhere for a keystroke to go until the Hold exists; and the pane
//! says so, because a terminal that silently swallows typing reads as broken
//! rather than as read-only.
//!
//! The grid and nothing above it: no scrollback here either, matching the server
//! that decided the repaint. A reader who wants everything the session printed
//! wants the Transcript beside this, or the Capture underneath it.

import { useQuery } from "@tanstack/solid-query";
import { Terminal } from "@xterm/xterm";
import { Match, Switch, createEffect, onCleanup, type JSX } from "solid-js";

import "@xterm/xterm/css/xterm.css";

import { loadScreen } from "../api/client";
import type { AgentOutputEvent, ConversationView } from "../api/types";

export function Screen(props: {
  conversation: ConversationView;
  output: AgentOutputEvent;
}): JSX.Element {
  const screen = useQuery(() => ({
    // The Event is in the key for the reason it is in the Transcript's: opening
    // another session's Screen is another query rather than this one showing
    // the wrong session's grid for a moment.
    queryKey: ["screen", props.conversation.id, props.output.id],
    queryFn: () => loadScreen(props.conversation.id, props.output.id),
  }));

  /// Where the terminal is mounted, and the terminal itself.
  let host: HTMLDivElement | undefined;
  let terminal: Terminal | undefined;

  createEffect(() => {
    const painted = screen.data;
    if (!painted || !host) {
      return;
    }

    // Made on the first repaint rather than up front, because a terminal has to
    // be told its size and the repaint is where the size comes from: the same
    // sequences put a session's display in different places on a grid of a
    // different width.
    if (!terminal) {
      terminal = new Terminal({
        cols: painted.columns,
        rows: painted.rows,
        // Nothing to type into, and nothing to type into it with — see the
        // note above.
        disableStdin: true,
        // The grid and nothing above it, as the server holds it.
        scrollback: 0,
        // The cursor is where the session left it, which is worth seeing and
        // not worth blinking: this is a still of a terminal, not one being
        // typed at.
        cursorBlink: false,
      });
      terminal.open(host);
    } else {
      terminal.resize(painted.columns, painted.rows);
    }

    // Cleared first, because a repaint says what the whole grid is rather than
    // what has changed about it — see the server's `screen` module. Written
    // over the top of the last one, it would be a screen with the last screen
    // showing through wherever this one has nothing to say.
    terminal.reset();
    terminal.write(painted.repaint);
  });

  // A terminal holds a parser, a buffer and its own listeners, none of which go
  // away with the element it drew into.
  onCleanup(() => terminal?.dispose());

  return (
    <Switch>
      <Match when={screen.isError}>
        <p class="error">
          Could not read this screen: {screen.error?.message}
        </p>
      </Match>
      <Match when={screen.data}>
        <div class="screen">
          <div class="terminal-host" ref={host} />
          <p class="note read-only">
            Read-only: this is what the session's terminal is showing.
          </p>
        </div>
      </Match>
      <Match when={true}>
        <p class="empty">Loading…</p>
      </Match>
    </Switch>
  );
}
