//! The window onto a terminal the server is holding: xterm, over a socket.
//!
//! Shared by the two things in the workbench that have one at the other end — a
//! session's **Screen**, which is the terminal an agent is drawing on, and a
//! Conversation's **Terminal**, which is a shell of the human's own inside its
//! Sandbox (ADR 0007 and [ADR
//! 0013](../../../docs/adr/0013-conversation-terminals.md)). Both are the same
//! server-held virtual terminal watched the same way, so both are drawn by this:
//! what runs at the far end is not something this side has to know.
//!
//! **The one deliberate exception to the rule that the browser never parses.** A
//! live terminal is a terminal rather than a document: shipping a rendered grid
//! down the wire would cost latency and bytes for no fidelity at all, and the
//! escape sequences are already the shorter description. What keeps the
//! exception bounded is that the server's virtual terminal stays the source of
//! truth — this one is handed a repaint and paints it, and never decides
//! anything.
//!
//! **How big the pane is goes back up the socket**, and the latest window wins
//! for everybody: there is one grid however many devices are watching it, and
//! the size reaches the terminal itself, so whatever is running on it redraws to
//! fit. What comes back is a repaint, which is the only thing that says how big
//! the grid now is — so the size is the server's answer rather than this one's
//! guess, exactly as the contents are.
//!
//! Both dimensions of it. The pane gives the terminal the room left under its
//! header and no more, so how many rows fit is as much a fact about this window
//! as how many columns are.
//!
//! **And so is what is put into it.** Keystrokes and mouse reports alike go up
//! the socket and straight into the terminal — xterm hands both out of the one
//! callback, and there is nothing here that has to tell them apart, because
//! neither commits Verkstead to anything. Nothing here draws the typing either:
//! a terminal's business is what it makes of a keystroke, and what it makes of
//! one comes back down the socket like everything else it prints.
//!
//! **A grid with nothing on it yet says so.** An empty black rectangle is
//! exactly what a terminal that has failed looks like, so until the first
//! repaint has landed the pane says it is waiting for one — which is the
//! difference between one that is slow to arrive and one that is not coming.
//!
//! The grid and nothing above it: no scrollback here either, matching the server
//! that decided the repaint.

import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { Show, createEffect, createSignal, onCleanup, type JSX } from "solid-js";

import "@xterm/xterm/css/xterm.css";

import type { Screen as Painted, Shown, Size, Watching } from "../api/types";
import { ErrorLine, Note } from "../notices";
import styles from "./Attached.module.css";
import shell from "../Panes.module.css";

/// What the pane says under the grid, which is the one thing the two callers
/// word differently: a Screen is a session being watched and a Terminal is a
/// shell being worked in.
export type Wording = {
  /// While nothing has been painted here yet.
  waiting: string;

  /// And once something has, or nothing at all where there is nothing worth
  /// saying about a grid that is simply working.
  watching?: string;

  /// And where the socket would not open, or closed under it.
  lost: string;
};

/// A terminal the server is holding, watched over `at` for as long as this is
/// drawn.
///
/// Always live and always typed into: a socket is a thing at the other end, and
/// one that had ended would have nothing to relay. What is read-only is
/// [`Standing`] below, which is a grid nothing will move again.
export function Attached(props: {
  at: string;
  say: Wording;

  /// A class of the caller's, for what the pane around it does with it —
  /// `paneWide` where the terminal is the whole point of the pane. Styled by
  /// whoever passes it, never here.
  class?: string;
}): JSX.Element {
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

  /// What went wrong, where something did. The socket's own failures: one that
  /// closes while there is still something at the other end is a thing to say in
  /// words.
  const [lost, setLost] = createSignal(false);

  /// Whether a grid has been painted here yet — see the note at the top.
  const [shown, setShown] = createSignal(false);

  /// Paint a whole grid, making the terminal if there is not one yet.
  const paint = (painted: Painted) => {
    if (!host) {
      return;
    }

    if (!terminal) {
      terminal = opened(painted, true);
      fit = new FitAddon();
      terminal.loadAddon(fit);
      terminal.open(host);

      // What a keystroke does. On the terminal rather than on the socket,
      // because it is the terminal that turns a keypress into the bytes the far
      // end expects — an arrow key, a control character, a pasted line — and
      // those bytes are what goes up. And what the mouse does, which comes out
      // of the same callback and goes up as the same kind of thing.
      terminal.onData((input) => putIn?.({ PutIn: input }));
    } else {
      terminal.resize(painted.columns, painted.rows);
    }

    repainted(terminal, painted);
    setShown(true);
  };

  createEffect(() => {
    const socket = new WebSocket(props.at);

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
        paint(shown.Painted);

        // Measured off the back of a repaint as well as on a resize, because
        // the first repaint is what makes the terminal: until it arrives there
        // is nothing here to measure the pane against.
        measure();
        return;
      }

      terminal?.write(shown.Printed);
    });

    // A socket that would not open, or that closed while there was still
    // something at the other end to watch.
    socket.addEventListener("error", () => setLost(true));

    /// How big this watcher has said its pane is. Nothing yet, until it has
    /// been measured — see [`measure`].
    let asked: Size | undefined;

    /// How big the pane is, in characters, said up the socket.
    ///
    /// Columns and rows both, because the pane gives the terminal the room left
    /// under the header and no more. The addon measures the element the
    /// terminal was opened in, which is the element the stylesheet has just
    /// sized, so both numbers are this window's own answer.
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
    <div
      class={[styles.screen, styles.live, shell.paneScreen, props.class]
        .filter(Boolean)
        .join(" ")}
    >
      <div class={styles.terminalHost} ref={host} />
      <Show
        when={lost()}
        fallback={
          <Show when={!shown() ? props.say.waiting : props.say.watching}>
            {(said) => <Note class={styles.note}>{said()}</Note>}
          </Show>
        }
      >
        <ErrorLine>{props.say.lost}</ErrorLine>
      </Show>
    </div>
  );
}

/// And a grid nothing will move again: painted once, from a repaint that was
/// fetched rather than watched.
///
/// Read-only, and it says so: a terminal that silently swallows typing reads as
/// broken rather than as read-only. Drawn at its own size and scrolling in the
/// card it sits on, because it is fixed at the size it was printed for — there
/// is nothing at the other end to redraw it.
export function Standing(props: { painted: Painted; say: string }): JSX.Element {
  let host: HTMLDivElement | undefined;
  let terminal: Terminal | undefined;

  createEffect(() => {
    const painted = props.painted;

    if (!host) {
      return;
    }

    if (!terminal) {
      terminal = opened(painted, false);
      terminal.open(host);
    } else {
      terminal.resize(painted.columns, painted.rows);
    }

    repainted(terminal, painted);
  });

  onCleanup(() => {
    terminal?.dispose();
  });

  return (
    <div class={`${styles.screen} ${shell.paneScreen}`}>
      <div class={styles.terminalHost} ref={host} />
      <Note class={styles.note}>{props.say}</Note>
    </div>
  );
}

/// An xterm the size the repaint says.
///
/// Made on the first repaint rather than up front, because a terminal has to be
/// told its size and the repaint is where the size comes from: the same
/// sequences put a display in different places on a grid of a different width.
///
/// `typing` is whether there is anything at the other end of a keystroke.
function opened(painted: Painted, typing: boolean): Terminal {
  return new Terminal({
    cols: painted.columns,
    rows: painted.rows,
    disableStdin: !typing,
    // The grid and nothing above it, as the server holds it.
    scrollback: 0,
    // The cursor is where the far end left it. Blinking on the one that can be
    // typed into and still on the one that cannot, because that is the
    // difference a reader is being shown.
    cursorBlink: typing,
  });
}

/// Put a whole grid on `terminal`.
///
/// Cleared first, because a repaint says what the whole grid is rather than what
/// has changed about it — see the server's `screen` module. Written over the top
/// of the last one, it would be a screen with the last screen showing through
/// wherever this one has nothing to say.
function repainted(terminal: Terminal, painted: Painted): void {
  terminal.reset();
  terminal.write(painted.repaint);
}
