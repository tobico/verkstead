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
//! **How big the pane is goes back up the socket**, and the latest window wins
//! for everybody: there is one Screen however many devices are watching it, and
//! the size reaches the session's own terminal, so its interface redraws to fit.
//! What comes back is a repaint, which is the only thing that says how big the
//! grid now is — so the size is the server's answer rather than this one's
//! guess, exactly as the contents are.
//!
//! **Both dimensions of it.** The rows were the server's to choose and the
//! columns this side's, on the reasoning that the pane was a column of a page
//! that scrolled and so had no height of its own to speak of. It has one now:
//! the Screen takes the pane whole, the terminal takes the room left under the
//! header, and nothing scrolls under it — so how many rows fit is as much a fact
//! about this window as how many columns are, and a session's interface lays
//! itself out against both.
//!
//! A session that has ended is where that stops. Its grid is the one it last
//! stood on and nothing will resize it again, so it is drawn at its own size and
//! scrolls inside the pane where the pane is the shorter of the two.
//!
//! **And so does what is put into it.** Keystrokes and mouse reports alike go up
//! the socket and straight into the session's own terminal — the terminal hands
//! both out of the one callback, and there is nothing here that has to tell them
//! apart, because neither commits Verkstead to anything. Typing into a driven
//! session changes nothing about when it ends; somebody who wants the run held
//! off while they work presses **Stop** first.
//!
//! Nothing here draws the typing either — a terminal's business is what it makes
//! of a keystroke, and what it makes of one comes back down the socket like
//! everything else it prints.
//!
//! A session that has ended is read-only, and says so: a terminal that silently
//! swallows typing reads as broken rather than as read-only.
//!
//! **And a Screen with nothing on it yet says that.** An empty black rectangle
//! is exactly what a terminal that has failed looks like, so until the first
//! repaint has landed the pane says it is waiting for one — which is the
//! difference between a Screen that is slow to arrive and a Screen that is not
//! coming.
//!
//! The grid and nothing above it: no scrollback here either, matching the server
//! that decided the repaint. A reader who wants everything the session printed
//! wants the Transcript beside this, or the Capture underneath it.

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
  Size,
  Watching,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import styles from "./Screen.module.css";
import shell from "../Panes.module.css";

export function Screen(props: {
  conversation: ConversationView;
  output: AgentOutputEvent;
}): JSX.Element {
  /// Whether this session is still printing, which is what decides where the
  /// Screen comes from — the socket or the fetch.
  const live = () => props.output.running;

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

  /// Where the terminal is mounted, the terminal itself, and the addon that
  /// measures how much of a grid fits in the pane.
  let host: HTMLDivElement | undefined;
  let terminal: Terminal | undefined;
  let fit: FitAddon | undefined;

  /// Where what the terminal makes of the human goes, or nothing where there is
  /// nowhere for it to go.
  ///
  /// Set by whatever is watching and read by the terminal's own listener, which
  /// is attached once when the terminal is made: a listener added per repaint
  /// would be one more copy of itself every time somebody resized a window.
  let putIn: ((said: Watching) => void) | undefined;

  /// What went wrong, where something did. The socket's own failures rather than
  /// the query's: a session that stops being watchable while somebody is
  /// watching is a thing to say in words.
  const [lost, setLost] = createSignal(false);

  /// Whether a grid has been painted here yet.
  ///
  /// Nothing arrives instantly and a terminal with nothing on it is a black
  /// rectangle, which is indistinguishable from one that is broken. So the pane
  /// says which it is until the first repaint lands — and a Screen that takes a
  /// noticeable while to arrive is then a Screen that was visibly on its way,
  /// rather than a pane that looked like it had failed.
  const [shown, setShown] = createSignal(false);

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
      // And what the mouse does, which comes out of the same callback: the
      // terminal reports a move, a click or a scroll to the session the way it
      // reports a keypress, and it goes up as the same kind of thing.
      //
      // Nothing is drawn here for either: what the session makes of it comes
      // back as what the session printed, which is the one account of what
      // happened.
      terminal.onData((input) => putIn?.({ PutIn: input }));
    } else {
      terminal.resize(painted.columns, painted.rows);
    }

    // Cleared first, because a repaint says what the whole grid is rather than
    // what has changed about it — see the server's `screen` module. Written
    // over the top of the last one, it would be a screen with the last screen
    // showing through wherever this one has nothing to say.
    terminal.reset();
    terminal.write(painted.repaint);
    setShown(true);
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

    // Where this pane's keystrokes and mouse reports go for as long as this
    // socket is the one watching.
    putIn = (said) => {
      if (socket.readyState !== WebSocket.OPEN) {
        return;
      }

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

    /// How big this watcher has said its pane is. Nothing yet, until it has
    /// been measured — see [`measure`].
    let asked: Size | undefined;

    /// How big the pane is, in characters, said up the socket.
    ///
    /// Columns and rows both, because the pane gives the terminal the room left
    /// under the header and no more — see the note at the top. The addon
    /// measures the element the terminal was opened in, which is the element the
    /// stylesheet has just sized, so both numbers are this window's own answer.
    ///
    /// Said only when *this pane* has changed shape, which is what `asked`
    /// remembers. Measured against the terminal's own size instead, two watchers
    /// of different shapes would argue: each would see the other's repaint arrive
    /// at the wrong size and ask for its own back, forever. The latest window
    /// wins by nobody re-asserting an older one.
    const measure = () => {
      if (!terminal || !fit || socket.readyState !== WebSocket.OPEN) {
        return;
      }

      const fits = fit.proposeDimensions();

      if (!fits?.cols || !fits.rows) {
        return;
      }

      if (fits.cols === asked?.columns && fits.rows === asked.rows) {
        return;
      }

      asked = { columns: fits.cols, rows: fits.rows };

      const resized: Watching = { Resized: asked };

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
      putIn = undefined;
    });
  });

  // A terminal holds a parser, a buffer and its own listeners, none of which go
  // away with the element it drew into.
  onCleanup(() => {
    terminal?.dispose();
  });

  return (
    <Switch>
      <Match when={screen.isError}>
        <ErrorLine>
          Could not read this screen: {screen.error?.message}
        </ErrorLine>
      </Match>
      <Match when={live() || screen.data}>
        {/* Whether the session is still printing, said on the element as well as
            asked in here: it is what decides whether the terminal scrolls. One
            still running is redrawn at whatever size this pane is, so there is
            nothing to scroll to; the grid one left behind is fixed at the size
            it was printed for. */}
        <div
          class={`${styles.screen} ${shell.paneScreen}`}
          classList={{ [styles.live!]: live() }}
        >
          <div class={styles.terminalHost} ref={host} />
          {/* What the human may do with this Screen, said under it. */}
          <Show
            when={lost()}
            fallback={
              <Note class={styles.readOnly}>
                {!shown()
                  ? "Waiting for this session's screen…"
                  : live()
                    ? "Watching. Type to work in this session — press Stop first if the run must not advance under you."
                    : "Read-only: this is what the session's terminal is showing."}
              </Note>
            }
          >
            <ErrorLine>
              The connection to this session's screen was lost.
            </ErrorLine>
          </Show>
        </div>
      </Match>
      <Match when={true}>
        <Empty>Loading…</Empty>
      </Match>
    </Switch>
  );
}
