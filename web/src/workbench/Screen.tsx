//! One session's Screen: the grid its terminal is showing, drawn as a terminal.
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
//! **A session that is still running is watched over a socket**: a repaint on
//! connect, and what it prints after that. The one socket in the app, and the
//! one place the viewer is sent something rather than fetching it — SSE and a
//! refetch stay the freshness model for everything else, a terminal being drawn
//! being the one thing neither of those is any good for. **A session that has
//! ended is fetched**, because its Screen is the one it last stood on and
//! nothing will move it again.
//!
//! **How wide the pane is goes back up the socket**, and the latest window wins
//! for everybody: there is one Screen however many devices are watching it, and
//! the size reaches the session's own terminal, so its interface redraws to fit.
//! What comes back is a repaint, which is the only thing that says how big the
//! grid now is — so the size is the server's answer rather than this one's
//! guess, exactly as the contents are.
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
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import {
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import "@xterm/xterm/css/xterm.css";

import { loadScreen, screenSocket } from "../api/client";
import type {
  AgentOutputEvent,
  ConversationView,
  Screen as Painted,
  Shown,
  Watching,
} from "../api/types";

export function Screen(props: {
  conversation: ConversationView;
  output: AgentOutputEvent;
}): JSX.Element {
  /// Whether this session is still printing, which is what decides where the
  /// Screen comes from — the socket or the fetch.
  const live = () => props.output.running;

  const screen = useQuery(() => ({
    // The Event is in the key for the reason it is in the Transcript's: opening
    // another session's Screen is another query rather than this one showing
    // the wrong session's grid for a moment.
    queryKey: ["screen", props.conversation.id, props.output.id],
    queryFn: () => loadScreen(props.conversation.id, props.output.id),
    // A running session is watched instead. One request for the grid as the
    // store last had it would be a request for something a repaint is about to
    // replace.
    enabled: !live(),
  }));

  /// Where the terminal is mounted, the terminal itself, and the addon that
  /// measures how many columns fit in the pane.
  let host: HTMLDivElement | undefined;
  let terminal: Terminal | undefined;
  let fit: FitAddon | undefined;

  /// What went wrong, where something did. The socket's own failures rather than
  /// the query's: a session that stops being watchable while somebody is
  /// watching is a thing to say in words.
  const [lost, setLost] = createSignal(false);

  /// Paint a whole grid, making the terminal if there is not one yet.
  ///
  /// Made on the first repaint rather than up front, because a terminal has to
  /// be told its size and the repaint is where the size comes from: the same
  /// sequences put a session's display in different places on a grid of a
  /// different width.
  const paint = (painted: Painted) => {
    if (!host) {
      return;
    }

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
        // not worth blinking: this is a window onto a terminal, not one being
        // typed at.
        cursorBlink: false,
      });

      fit = new FitAddon();
      terminal.loadAddon(fit);
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
  };

  // The session that has ended: the grid it left, fetched.
  createEffect(() => {
    const painted = screen.data;

    if (painted && !live()) {
      paint(painted);
    }
  });

  // And the session that is still running: the same grid, watched.
  createEffect(() => {
    if (!live()) {
      return;
    }

    const socket = new WebSocket(
      screenSocket(props.conversation.id, props.output.id),
    );

    socket.addEventListener("message", (event: MessageEvent<string>) => {
      const shown: Shown = JSON.parse(event.data);

      if ("Painted" in shown) {
        paint(shown.Painted);

        // Measured off the back of a repaint as well as on a resize, because
        // the first repaint is what makes the terminal: until it arrives there
        // is nothing here to measure the pane against.
        measure();
        return;
      }

      terminal?.write(shown.Printed);
    });

    // A socket that would not open, or that closed while the session was still
    // said to be running. Not the ordinary end of one: a session that finishes
    // closes this and says so on the Timeline a moment later, which is what
    // moves the pane over to the fetch.
    socket.addEventListener("error", () => setLost(true));

    /// How wide this watcher has said its pane is. Nothing yet, until it has
    /// been measured — see [`measure`].
    let asked = 0;

    /// How wide the pane is, in characters, said up the socket.
    ///
    /// The columns and not the rows: the pane is a column of the page that
    /// scrolls, so its height is the reader's window rather than the Screen's,
    /// and the grid keeps however many rows the server says it has. What a
    /// terminal application lays itself out against is the width.
    ///
    /// Said only when *this pane* has changed shape, which is what `asked`
    /// remembers. Measured against the terminal's own width instead, two
    /// watchers of different sizes would argue: each would see the other's
    /// repaint arrive at the wrong width and ask for its own back, forever. The
    /// latest window wins by nobody re-asserting an older one.
    const measure = () => {
      if (!terminal || !fit || socket.readyState !== WebSocket.OPEN) {
        return;
      }

      const fits = fit.proposeDimensions();

      if (!fits?.cols || fits.cols === asked) {
        return;
      }

      asked = fits.cols;

      const resized: Watching = {
        Resized: { columns: fits.cols, rows: terminal.rows },
      };

      socket.send(JSON.stringify(resized));
    };

    // Measured once the socket is up — the pane is drawn by then, and the size
    // it opens at is as much a resize as any later one — and again whenever the
    // pane changes shape under it.
    socket.addEventListener("open", () => {
      setLost(false);
      measure();
    });

    const watching = new ResizeObserver(measure);

    if (host) {
      watching.observe(host);
    }

    onCleanup(() => {
      watching.disconnect();
      socket.close();
    });
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
      <Match when={live() || screen.data}>
        <div class="screen">
          <div class="terminal-host" ref={host} />
          <Show
            when={lost()}
            fallback={
              <p class="note read-only">
                {live()
                  ? "Watching. Read-only: there is nowhere here to type."
                  : "Read-only: this is what the session's terminal is showing."}
              </p>
            }
          >
            <p class="error">
              The connection to this session's screen was lost.
            </p>
          </Show>
        </div>
      </Match>
      <Match when={true}>
        <p class="empty">Loading…</p>
      </Match>
    </Switch>
  );
}
