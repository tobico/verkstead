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
//! **And what the far end calls itself comes back out.** A shell sets a title
//! the way it prints anything else, and xterm reads it out of the escape —
//! which is the one thing a window learns about what is running on it. There is
//! nowhere on a grid for a title, so it is handed to whoever drew this window
//! instead: a Terminal's tabs are named by it, and a Screen has no name to draw
//! and asks for none.
//!
//! **A grid with nothing on it yet says so.** An empty black rectangle is
//! exactly what a terminal that has failed looks like, so until the first
//! repaint has landed the pane says it is waiting for one — which is the
//! difference between one that is slow to arrive and one that is not coming.
//!
//! **Several of these may stand over one pane**, which is what a Conversation's
//! Terminal does with its tabs: every one of them keeps its socket and its grid,
//! and only the one showing is drawn. Which is why `showing` is handed in rather
//! than read off this element — the window showing is the only one that measures
//! the pane, and two of them measuring one pane would be the oscillation the
//! de-dupe below guards against, with a hidden window's nothing always winning.
//!
//! **What is above the grid is the caller's.** The server holds the grid alone,
//! so a repaint is where a window starts and there is nothing to fetch a
//! scrollback from — but a window can keep what it watched go past, and whether
//! it is worth keeping is a question about what is at the far end. A session's
//! Screen keeps none: what it is watching is an agent's interface redrawing
//! itself, and everything it ever printed is in the Transcript beside it and
//! the Capture under it. A Terminal keeps `scrollback` lines, because a human
//! who has just run a build in one wants to read it, and nothing else here
//! wrote it down.

import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import {
  Match,
  Switch,
  createEffect,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

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
/// Live while there is something at the far end of the socket, and read-only
/// once there is not: a caller that keeps such a window past its shell says so
/// with `over`, and the grid the shell left stands there taking no typing.
/// [`Standing`] below is the other read-only grid — one that was fetched rather
/// than watched, and was never live at all.
export function Attached(props: {
  at: string;
  say: Wording;

  /// A class of the caller's, for what the pane around it does with it —
  /// `paneWide` where the terminal is the whole point of the pane. Styled by
  /// whoever passes it, never here.
  class?: string;

  /// Whether this is the window showing, where the caller draws more than one
  /// over the one pane. Absent where it draws one, a Screen being the whole of
  /// its half of the switch above it.
  ///
  /// Three things follow from it, and all three are about a window nobody can
  /// see: it is hidden rather than laid out, it does not measure the pane — see
  /// the note at the top — and it does not take the typing. Which is why the
  /// focus is here rather than in the caller: the terminal is made by its first
  /// repaint, so a caller reaching in for one would be reaching for something
  /// that may not have arrived yet.
  showing?: boolean;

  /// How many lines this window keeps above the grid, or none where the caller
  /// says nothing — see the note at the top, which is where the two callers
  /// differ.
  ///
  /// The window's own and no part of what the server holds: a repaint clears it,
  /// because a repaint says what the whole grid is and the lines above it are
  /// this window's memory of watching rather than anything the far end can
  /// account for. So a reload, or a second device, starts at the grid.
  scrollback?: number;

  /// The far end has gone, and this is the line to stand under the grid saying
  /// so — the shell ended, or its open was refused.
  ///
  /// The grid stays exactly where it stopped and goes read-only with it: a
  /// terminal that silently swallows typing reads as broken rather than as
  /// over, which is the difference [`Standing`] below is drawn under too.
  ///
  /// Only a caller that keeps such a window says anything here. A Screen's does
  /// not: a session that has ended is drawn from its Capture by the pane above
  /// it rather than left attached to nothing.
  over?: string;

  /// What the shell at the far end calls itself, said again every time it
  /// changes it — the terminal title escape, which is what a prompt sets at
  /// every prompt.
  ///
  /// Reported rather than drawn, because there is nowhere on a grid for a title
  /// to go: what is made of it belongs to whatever put this window on a pane,
  /// and for a Conversation's Terminal that is the tab's label. Empty where the
  /// shell cleared it, which is a name taken back rather than a name of no
  /// letters.
  ///
  /// Nothing is said until the shell says something: a repaint carries the grid
  /// and not the title, so a window that has just attached knows no more about
  /// what is running on it than it did before.
  titled?: (title: string) => void;

  /// And what to call when the socket closes with this window still drawn,
  /// which is what a shell ending looks like from here: the server takes the
  /// terminal off its register and every watcher's socket closes with it.
  ///
  /// This says what happened rather than doing anything about it, because what
  /// follows from it is the caller's. A shell that ran for an hour and one that
  /// never started are the same closed socket, and only the pane knows which it
  /// asked for and when — see `Terminal.tsx`, where the five seconds are.
  ended?: () => void;
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

  /// And how this window says how big it is, for the moment it becomes the one
  /// showing. Set by whatever is watching, for the reason [`putIn`] is: a size
  /// goes up the socket that is open now.
  let measuring: (() => void) | undefined;

  /// What went wrong, where something did. The socket's own failures: one that
  /// closes while there is still something at the other end is a thing to say in
  /// words.
  const [lost, setLost] = createSignal(false);

  /// Whether a grid has been painted here yet — see the note at the top.
  const [shown, setShown] = createSignal(false);

  /// The typing, where this is the window showing. Nothing where the caller
  /// said nothing about showing: a Screen being opened is no reason to take the
  /// focus off whatever had it, and one window over a pane is nobody's tab.
  const taking = () => {
    if (props.showing === true) {
      terminal?.focus();
    }
  };

  /// Paint a whole grid, making the terminal if there is not one yet.
  const paint = (painted: Painted) => {
    if (!host) {
      return;
    }

    if (!terminal) {
      terminal = opened(painted, true, props.scrollback ?? 0);
      fit = new FitAddon();
      terminal.loadAddon(fit);
      terminal.open(host);

      // What a keystroke does. On the terminal rather than on the socket,
      // because it is the terminal that turns a keypress into the bytes the far
      // end expects — an arrow key, a control character, a pasted line — and
      // those bytes are what goes up. And what the mouse does, which comes out
      // of the same callback and goes up as the same kind of thing.
      terminal.onData((input) => putIn?.({ PutIn: input }));

      // And what the shell calls itself, for whoever is drawing a name for this
      // window. On the terminal for the reason the typing is: it is xterm that
      // reads a title out of the escape the shell printed, and this side only
      // passes on what it read.
      terminal.onTitleChange((title) => props.titled?.(title));

      // And the typing, where this is the window showing. Here as well as in
      // the effect below, because the two are the two ways a window comes to be
      // the one showing: turned to, which the effect answers, and opened
      // already showing — a tab plus has just added — where there is nothing to
      // focus until this repaint has made it.
      taking();
    } else {
      terminal.resize(painted.columns, painted.rows);
    }

    repainted(terminal, painted);
    setShown(true);
  };

  createEffect(() => {
    const socket = new WebSocket(props.at);

    /// Whether this window is the one closing the socket, in which case its
    /// closing is nothing to report: a socket closed on the way out is this
    /// element going away rather than the far end of it.
    let leaving = false;

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

    // And one the server closed under this window, which is the far end gone:
    // a shell that exited, or a number that was never live to begin with.
    socket.addEventListener("close", () => {
      if (leaving) {
        return;
      }

      props.ended?.();
    });

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
      if (
        props.showing === false ||
        !terminal ||
        !fit ||
        socket.readyState !== WebSocket.OPEN
      ) {
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

    measuring = measure;

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
      leaving = true;
      watching.disconnect();
      socket.close();
      putIn = undefined;
      measuring = undefined;
    });
  });

  // Over: the grid is where the shell got to rather than something to type
  // into, and it says so by not taking the typing — the same difference
  // [`opened`] draws for a grid that was fetched rather than watched.
  createEffect(() => {
    if (props.over === undefined || !terminal) {
      return;
    }

    terminal.options.disableStdin = true;
    terminal.options.cursorBlink = false;
  });

  // Turned to: the pane is this window's now, so it measures it and takes the
  // typing. Nothing at all where the caller said nothing about showing — one
  // window over a pane measures off its socket and its observer as it always
  // did.
  createEffect(() => {
    if (props.showing !== true) {
      return;
    }

    measuring?.();
    taking();
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
      hidden={props.showing === false}
    >
      <div class={styles.terminalHost} ref={host} />
      <Switch>
        {/* What is over is over, whichever way the socket went — a shell that
            exited closed it cleanly and one that never started failed it, and
            the caller has already said which of those this was. */}
        <Match when={props.over}>
          {(said) => <ErrorLine>{said()}</ErrorLine>}
        </Match>
        <Match when={lost()}>
          <ErrorLine>{props.say.lost}</ErrorLine>
        </Match>
        <Match when={!shown() ? props.say.waiting : props.say.watching}>
          {(said) => <Note class={styles.note}>{said()}</Note>}
        </Match>
      </Switch>
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
      terminal = opened(painted, false, 0);
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
/// `typing` is whether there is anything at the other end of a keystroke, and
/// `keeping` how many lines this window holds above the grid — see the note at
/// the top, where the difference between the two callers is.
function opened(painted: Painted, typing: boolean, keeping: number): Terminal {
  return new Terminal({
    cols: painted.columns,
    rows: painted.rows,
    disableStdin: !typing,
    scrollback: keeping,
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
