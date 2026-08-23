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
//! **And so does what is typed into it.** The first keystroke takes the Hold:
//! the human is at the keyboard, Verkstead stops ending sessions and advancing
//! runs, and the Conversation carries *blocked on you* until the keyboard is
//! handed back. Nothing here draws the typing — a terminal's business is what it
//! makes of a keystroke, and what it makes of one comes back down the socket
//! like everything else it prints.
//!
//! **Handing back is a press, and it is the only way out.** Not closing the
//! pane, not closing the tab: Verkstead resuming over a half-finished
//! intervention is worse than a stalled run. So the control sits here for as
//! long as the Hold does — on a session that has exited as much as on one still
//! running, because a session that exited held is exactly one still waiting to
//! be judged.
//!
//! A session that has ended and is not held is read-only, and says so: a
//! terminal that silently swallows typing reads as broken rather than as
//! read-only.
//!
//! The grid and nothing above it: no scrollback here either, matching the server
//! that decided the repaint. A reader who wants everything the session printed
//! wants the Transcript beside this, or the Capture underneath it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
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

import { handBack, loadScreen, screenSocket } from "../api/client";
import type {
  AgentOutputEvent,
  ConversationView,
  Screen as Painted,
  Shown,
  Watching,
} from "../api/types";
import { useReading } from "../freshness";

export function Screen(props: {
  conversation: ConversationView;
  output: AgentOutputEvent;
}): JSX.Element {
  /// Whether this session is still printing, which is what decides where the
  /// Screen comes from — the socket or the fetch.
  const live = () => props.output.running;

  /// Whether the human has this session's keyboard.
  ///
  /// The server's answer rather than this page's memory of having typed: one
  /// Hold is one Conversation's, and a phone that took it is a Hold this laptop
  /// has to be able to see and hand back.
  const held = () => props.conversation.held === props.output.id;

  const screen = useReading(() => ({
    // The Event is in the key for the reason it is in the Transcript's: opening
    // another session's Screen is another query rather than this one showing
    // the wrong session's grid for a moment.
    queryKey: ["screen", props.conversation.id, props.output.id],
    queryFn: () => loadScreen(props.conversation.id, props.output.id),
    // A running session is watched instead. One request for the grid as the
    // store last had it would be a request for something a repaint is about to
    // replace.
    enabled: !live(),

    // Which is why this one is frozen rather than merged: it is only ever
    // fetched for a session that has stopped, and a stopped session's Screen is
    // the grid it last stood on and nothing after it. A session running when
    // the pane opened is watched over the socket until it ends, and the one
    // read that follows is the read of a record that cannot move again.
    freshness: "static",
  }));

  const queries = useQueryClient();

  /// Give the keyboard back, and read the Conversation again: the badge, this
  /// pane and whatever else the Hold was showing are all drawn off it.
  const handingBack = useMutation(() => ({
    mutationFn: () => handBack(props.conversation.id),
    onSuccess: () => {
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  /// Where the terminal is mounted, the terminal itself, and the addon that
  /// measures how many columns fit in the pane.
  let host: HTMLDivElement | undefined;
  let terminal: Terminal | undefined;
  let fit: FitAddon | undefined;

  /// Where a keystroke goes, or nothing where there is nowhere for one to go.
  ///
  /// Set by whatever is watching and read by the terminal's own listener, which
  /// is attached once when the terminal is made: a listener added per repaint
  /// would be one more copy of itself every time somebody resized a window.
  let typed: ((keys: string) => void) | undefined;

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
  const paint = (painted: Painted, typing: boolean) => {
    if (!host) {
      return;
    }

    if (!terminal) {
      terminal = new Terminal({
        cols: painted.columns,
        rows: painted.rows,
        // A live session takes what is typed into it — see the note above, and
        // `onData` below, which is what carries it. A session that has ended has
        // no terminal at the other end of a keystroke.
        disableStdin: !typing,
        // The grid and nothing above it, as the server holds it.
        scrollback: 0,
        // The cursor is where the session left it. Blinking on the one that can
        // be typed into and still on the one that cannot, because that is the
        // difference a reader is being shown.
        cursorBlink: typing,
      });

      fit = new FitAddon();
      terminal.loadAddon(fit);
      terminal.open(host);

      // What a keystroke does. On the terminal rather than on the socket,
      // because it is the terminal that turns a keypress into the bytes a
      // session expects — an arrow key, a control character, a pasted line — and
      // those bytes are what goes up.
      //
      // Nothing is drawn here for it: what the session makes of it comes back as
      // what the session printed, which is the one account of what happened.
      terminal.onData((keys) => typed?.(keys));
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
      paint(painted, false);
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

    // Where this pane's keystrokes go for as long as this socket is the one
    // watching. The first of them takes the Hold, which is the server's to
    // decide and to say: nothing here assumes it was taken.
    typed = (keys) => {
      if (socket.readyState !== WebSocket.OPEN) {
        return;
      }

      const said: Watching = { Typed: keys };
      socket.send(JSON.stringify(said));
    };

    socket.addEventListener("message", (event: MessageEvent<string>) => {
      const shown: Shown = JSON.parse(event.data);

      if ("Painted" in shown) {
        paint(shown.Painted, true);

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
      typed = undefined;
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
          {/* What the human may do with this Screen, said under it. A Hold
              outranks everything else there is to say: it is the one thing here
              that has stopped the work, and the way out of it is the press
              beside the words. */}
          <Show
            when={held()}
            fallback={
              <Show
                when={lost()}
                fallback={
                  <p class="note read-only">
                    {live()
                      ? "Watching. Type to take the keyboard — Verkstead stops until you hand it back."
                      : "Read-only: this is what the session's terminal is showing."}
                  </p>
                }
              >
                <p class="error">
                  The connection to this session's screen was lost.
                </p>
              </Show>
            }
          >
            <div class="holding">
              <p class="note">
                {live()
                  ? "You have the keyboard. Verkstead is recording and nothing else."
                  : "You have the keyboard, and the session has exited. Nothing is judged until you hand it back."}
              </p>
              <button
                type="button"
                class="hand-back"
                disabled={handingBack.isPending}
                onClick={() => handingBack.mutate()}
              >
                Hand back
              </button>
            </div>
          </Show>
        </div>
      </Match>
      <Match when={true}>
        <p class="empty">Loading…</p>
      </Match>
    </Switch>
  );
}
