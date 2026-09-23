//! **Code**: the Conversation's editor — its files, and its terminals beside
//! them.
//!
//! Shells of the human's inside its Sandbox, with its Worktree as the working
//! directory — for the moment the agent's work is done and somebody wants to
//! try it, make a small change or work with git, without leaving the workbench
//! and without the run noticing
//! ([ADR 0013](../../../docs/adr/0013-conversation-terminals.md)).
//!
//! **The pane the Terminal pane became**
//! ([ADR 0019](../../../docs/adr/0019-the-code-pane.md)). Two panes, one
//! holding shells and one holding shells and files, would be the same thing
//! drawn twice — so there is no Terminal pane any more, and the path it stood
//! at redirects here (see `App.tsx`).
//!
//! **A tree down the side and a group of tabs beside it**, which is the shape
//! of the whole pane. The tree is [`./Tree`]: a root per Worktree the
//! Conversation has, one folder read when it is expanded. The group is what is
//! in this file, and it holds files and terminals alike — one bar, one set of
//! tabs, because a shell and a file are two things to have open rather than two
//! kinds of pane (ADR 0019, *Tabs and groups*).
//!
//! **A file pressed in the tree opens as a tab of this group.** What the server
//! answers a read with says which of four kinds of thing it read — text, an
//! image, a binary it will not send, or a file over the size cap — and the tab
//! draws each as what it is: the text in an editor, the picture in its tab, and
//! either of the last two as the line saying why there is nothing to draw. A
//! file in a read-only root opens read-only and takes no typing, which is the
//! root's own flag rather than the file's mode.
//!
//! **The editor is Monaco**, whole, in [`./Editor`]: every built-in language
//! coloured and the four bundled services answering, fetched as a chunk of its
//! own the first time this pane opens and never before (ADR 0019, *Monaco,
//! whole*). This pane's part in that is the `load` below, which is the fetch
//! starting because Code was opened rather than because a file was pressed.
//!
//! The buffer is what the human's text is in and the reading is what the disk
//! said — two things rather than one, because the save of the next task is a
//! comparison between them over the version the read carried.
//!
//! **And the same file opened twice is one buffer.** There is one group in this
//! stage, so that means one tab: pressing a file already open turns to its tab
//! rather than opening a second beside it.
//!
//! Opened by the code icon on the Timeline's header — see `Timeline.tsx` —
//! which is a details pane like every other, at a path of its own so it survives
//! a reload and can be linked to. The second pane nothing on the record opens: a
//! terminal belongs to the Conversation rather than to any moment on it, the way
//! sharing does.
//!
//! **The Screen's own viewer, filling the group.** What is drawn in a terminal
//! tab is [`./Attached`], the same xterm over the same socket to the same
//! server-held virtual terminal a session's Screen is watched through — what
//! runs on this one is a shell rather than an agent, and that is the whole of
//! the difference. The pane gives the pair every inch it has: the reading
//! measure every other details pane pads its content to comes off, the way the
//! composer takes it off, and the pane ends where the window does, so the tree
//! and the terminal are sized to the pane rather than scrolling it.
//!
//! **Several of them, one per tab.** The bar in the pane's header holds a tab
//! per thing open, in the order they were opened, and a plus at the end opens
//! another terminal. It is the Output pane's Transcript/Screen switch built
//! again — pressed-or-not buttons in a group rather than a tablist, which is the
//! house's answer to this shape — restyled after VS Code's bar, which is what
//! the pane is drawn after from here on: a kind icon at one end of every tab and
//! a × at the other, and the tabs abutting rather than spaced. The kind is the
//! whole of what an icon there says, and there are two of them: a shell, and a
//! file.
//!
//! The bar is drawn where there are tabs to draw. A strip holding nothing but
//! its own plus is furniture about tabs that are not there, and a pane with
//! nothing open has the hint under it to say the same thing in words.
//!
//! **And a file's tab is called what the file is.** Its name rather than its
//! path: a tab is a few rems wide and a path in a checkout is a sentence, and
//! the tree beside it is where a file is found by where it sits.
//!
//! **And a terminal's tab is called what its shell calls itself.** A prompt sets the
//! terminal's title at every prompt — the directory it is in, the command it is
//! running — and that is a better name for a tab than anything this side could
//! invent, so xterm's reading of the title escape is the label. Where the shell
//! has set none, or has cleared the one it set, the tab falls back to
//! *Terminal N* by the number the server gave it, which is why those numbers are
//! never reused.
//!
//! A fresh attach starts from the number again: a repaint carries the grid and
//! not the title, so a pane that has just loaded knows the number and nothing
//! else until the shell next says a name — which, for most prompts, is the next
//! time it draws one.
//!
//! Every tab keeps its socket open and its grid mounted whether or not it is the
//! one showing, so a shell that printed while somebody was reading another tab
//! has printed it by the time they turn back. Only the tab showing measures the
//! pane and says so up its socket — see [`./Attached`], where the hiding, the
//! measuring and the focus all are.
//!
//! **The server holds the shells, so this pane is the way back to them rather
//! than where they live.** On load it asks which of the Conversation's terminals
//! are live and draws a tab for each — so a reload, a second device or a tab
//! closed by accident comes back to what was already there, still running and
//! showing what it last showed.
//!
//! **And a tab is closed by the × at its end** — a file's as much as a shell's,
//! the file's taking its reading and its buffer with it, so that opening it
//! again is a fresh reading of the disk the way expanding a folder is. ADR 0013 kept Close on a context
//! menu, a × beside a label this small being a thing to hit by accident and what
//! it would end a shell somebody is working in; ADR 0019 puts it on the tab,
//! because the × is what VS Code's bar has, what carries that worry now is the
//! confirm a *busy* shell asks for, and a long press with no menu behind it is
//! what frees the gesture for dragging a tab. The press asks the server to end
//! that shell, and the tab then goes the way every ended shell's tab goes: its
//! socket closes, and the tab closes with it.
//!
//! **And the confirm is the server's answer rather than this side's reading.**
//! Whether somebody is working in a shell is something only the server can see
//! — it holds the pseudo-terminal, and what is in front of one is read off that
//! — so the press goes out, and a shell with something other than itself in the
//! foreground comes back *busy* with nothing done. That is what puts the card
//! up; the press inside it goes out again saying the human was asked, and ends
//! the shell whatever is running. Which is why the list's own flag is not what
//! is consulted: it says what was running when the pane loaded, and a build
//! started since is a build. Where the server cannot tell — a pseudoconsole on
//! Windows has no foreground process group — it says busy, so every close there
//! asks.
//!
//! **And the pane opens empty.** It opens no shell of its own accord: the live
//! ones come back as tabs, and where there are none it draws a hint and a **New
//! terminal** button where a tab's content goes. The Terminal pane never stood
//! empty because a shell was the whole of what it held; a pane that holds files
//! has something to show without one, and a shell nobody asked for is a
//! Sandbox started by the opening of a pane. A shell that exits closes its
//! socket, which is how this side hears about it: the tab goes, and where it was
//! the last the pane stands empty again.
//!
//! The guard on a shell that could not start stays, and is time — a shell that
//! ended within [`AT_ONCE`] of being asked for, or that the server refused to
//! open at all, is one that could not start rather than one that ran, so its tab
//! *stays* saying why, and **New terminal** replaces it. Without it a Sandbox
//! that will not start would leave an empty pane and nothing to read about why.
//! The clock is this pane's own, started when it asked: the server opens nothing
//! of its own accord and knows nothing about tabs.
//!
//! Nothing here is a record: no Capture, no Event on the Timeline, nothing in a
//! Share. And nothing here holds the run off — typing into a terminal is the
//! human doing something, exactly as typing into a Screen is, and somebody who
//! means to take the work on presses **Stop** first.

import {
  faFile,
  faPlus,
  faTerminal,
  faXmark,
} from "@fortawesome/free-solid-svg-icons";
import { useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  onCleanup,
  type JSX,
} from "solid-js";

import { Icon } from "../Icon";
import { IconButton } from "../IconButton";
import { Modal } from "../Modal";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
import {
  closeTerminal,
  listTerminals,
  openTerminal,
  readFile,
  terminalSocket,
} from "../api/client";
import type {
  ConversationView,
  FileReading,
  TerminalOpened,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Attached } from "./Attached";
import { Editor } from "./Editor";
import { load } from "./editing";
import { PaneHead } from "./PaneHead";
import { Tree } from "./Tree";
import styles from "./Code.module.css";
import shell from "../Panes.module.css";

/// Each way of being refused a terminal, in the words of what to go and do
/// about it.
///
/// One line each rather than a single "could not open one", because the server
/// names them separately for exactly this: a worktree that is not there, a
/// profile nobody chose and a shell that would not start are three different
/// jobs, and only the human can tell which they are looking at.
///
/// The first two are what the icon beside Share is already drawn against, said
/// again here because a pane is drawn against a Conversation that may have
/// moved since it was read.
export const TERMINAL_REFUSAL: Record<
  Extract<TerminalOpened, string>,
  string
> = {
  NoSuchConversation: "This conversation is gone.",
  NoWorktree: "This conversation has no worktree to open a terminal in.",
  NoProfile:
    "This conversation has no agent profile settled, so there is no account to run a shell under.",
  Refused: "The shell would not start. The server's log says why.",
};

/// How many lines a tab keeps above the grid, so that what scrolled past can be
/// read back.
///
/// The server holds the grid alone — a terminal is memory only, and a repaint is
/// the whole of what a fresh attach is given (ADR 0013) — so this is the tab's
/// own memory of watching and goes when the tab does. A Screen keeps none, and
/// is right to: what an agent printed is in the Transcript beside it and the
/// Capture under it. Nothing writes down what a shell printed, and a human who
/// has just run a build in one wants to read it.
///
/// Enough for a build or a test run and not much more, because it is held per
/// tab and a pane may have several. What a repaint clears with it is this and
/// nothing of the server's — see [`./Attached`].
export const SCROLLBACK = 5_000;

/// How soon after being asked for a shell has to end for its tab to stay.
///
/// The line between a shell that ran and a shell that could not start, and there
/// is no better one to draw: what comes back from the server is a terminal that
/// opened, and a shell that dies on its first line dies after the answer. Five
/// seconds is long enough that nothing a human did in the terminal is inside it
/// — a `exit` typed by hand takes longer than that to type — and short enough
/// that a pane sitting behind a broken Sandbox says so at once.
///
/// Measured from when *this pane* asked, because this pane is the only thing
/// that knows: the server opens nothing of its own accord and holds no clock.
export const AT_ONCE = 5_000;

/// What a tab says when its shell ended inside that.
///
/// The grid it left stands under it, read-only, because whatever the shell
/// managed to print on its way out is the only thing here that says why — and
/// the press that would try again is named, since nothing is going to try on its
/// own.
export const ENDED_AT_ONCE =
  "The shell ended as soon as it started. Press New terminal to open another.";

/// And what the pane says when it is holding nothing at all.
///
/// The state the Terminal pane never had. What it says is the two ways there
/// are into this group: a file out of the tree beside it, and a shell of the
/// human's own in the Worktree. The press under it opens the second, the first
/// being a press on something already drawn.
export const NOTHING_OPEN =
  "Nothing is open. Press a file in the tree to edit it, or open a terminal — a shell of your own in this conversation's worktree.";

/// Each way a file can come back with nothing to draw, in the words of what it
/// is.
///
/// One sentence each rather than a single "could not be opened", for the reason
/// the tree's `FOLDER_REFUSAL` has one apiece: the server names them separately
/// because each is a different thing for the human to do about it, and only
/// they can tell which they are looking at.
///
/// The first two are not refusals at all — the file is there and the server
/// read it — they are the two kinds this pane does not draw. They are worded
/// the same way for the same reason: what the tab has to say is why there is
/// nothing in it.
///
/// **The cap is named** rather than left as "too large", the way the composer's
/// own refusal names it — see `ATTACH_REFUSAL` in `Composer.tsx`. It is
/// `MAX_BYTES` in the server's `files`, and a round number in the words this
/// product says sizes in so that this sentence can carry it.
export const FILE_REFUSAL: Record<Extract<FileReading, string>, string> = {
  Binary: "This file is not text, so there is nothing here to edit.",
  TooLarge:
    "This file is larger than 2 MB, so Code does not open it. A terminal will.",
  Outside: "That is not in any of this conversation's worktrees.",
  UnderGit: "Code does not show what is inside a repository's .git.",
  RootGone: "This worktree is no longer on disk.",
  Missing: "That file is no longer there.",
  NotAFile: "That is a folder rather than a file.",
};

/// One tab of the group: a terminal by the number the server issued it, or a
/// file by its path.
///
/// Two shapes rather than one with a kind beside it, because the two are named
/// by different things and nothing here ever has to ask a tab what it is
/// without then using the answer.
export type Tab = { terminal: number } | { file: string };

/// What a tab is known by, which is the one string that can stand for either.
///
/// Which tab is showing, and which the human turned to, are both this rather
/// than the tab itself: a path and a number would be two signals holding one
/// answer between them.
export function keyed(tab: Tab): string {
  return "terminal" in tab ? `terminal ${tab.terminal}` : `file ${tab.file}`;
}

/// And what a file's tab is called: its name.
///
/// Rather than its path — a tab is a few rems wide, a path in a checkout is a
/// sentence, and where a file sits is what the tree beside it says. A path of
/// nothing but separators has no name, and reads as itself.
export function named(path: string): string {
  return path.split(/[/\\]/).filter(Boolean).pop() ?? path;
}

export function Code(props: {
  conversation: ConversationView;
  back: () => void;
}): JSX.Element {
  /// Which of this Conversation's terminals are live.
  ///
  /// Read when the pane opens rather than followed: the register is the
  /// server's and moves only when a shell exits or this pane opens one, and both
  /// of those are answered where they happen — an exit down the tab's own
  /// socket. Frozen for that reason — a terminal is no part of the record, so no
  /// Nudge is ever about one.
  ///
  /// **And kept no longer than the pane it was read for.** Frozen means frozen:
  /// an answer still in the cache is never asked for again, on a mount or on a
  /// focus. So a pane coming back to a cached one would be reading the register
  /// as it stood when this pane last closed — and where the shell was opened in
  /// this pane, that is the empty list it started from, so it would come back to
  /// no tabs at all and leave the shell running with nothing over it and no way
  /// to close it. Dropped with the pane instead, which is what makes "read when
  /// the pane opens" true of every opening rather than of the first.
  ///
  /// Said twice, because `gcTime` is a timer: it is zero, so nothing is kept
  /// past the pane, and the answer is dropped outright as the pane goes so that
  /// a pane reopened before that timer has run — which is a press that swaps one
  /// details pane for another, both in the one tick — finds nothing to come
  /// back to either.
  const held = useQueryClient();
  const holding = () => ["terminals", props.conversation.id];

  const terminals = useReading(() => ({
    queryKey: holding(),
    queryFn: () => listTerminals(props.conversation.id),
    freshness: "static",
    gcTime: 0,
  }));

  onCleanup(() => held.removeQueries({ queryKey: holding(), exact: true }));

  // And the editor, fetched because this pane is open and for no other reason
  // (ADR 0019, *Monaco, whole*). Here rather than in the tab that will want it,
  // so that the chunk is usually already in the browser by the time somebody
  // has picked a file to put in it — and here rather than anywhere above this
  // pane, because a workbench that never opens Code never fetches a byte of it.
  //
  // Nothing waits on it and nothing is drawn about it: the tab that opens a
  // file asks again, which is the one that has somewhere to say so. The call is
  // the one fetch either way — see [`./editing`].
  void load().catch(() => {});

  /// Every tab there is, in the order they were opened: what was live when the
  /// pane loaded, and what has been opened since — files and terminals alike,
  /// in the one order, because they are the one bar.
  ///
  /// The server issues its numbers in order and never reuses one, so the list it
  /// answers with is already in that order and everything opened here goes on
  /// the end. A tab standing on a shell that never started is one of these too,
  /// under a key of its own — see [`stand`].
  const [tabs, setTabs] = createSignal<Tab[]>([]);

  /// What each open file came back as: the text and its version, the picture,
  /// or the line saying why there is nothing to draw. Nothing at all while the
  /// read is in flight, which is the moment between the press and the answer.
  const [readings, setReadings] = createSignal<Record<string, FileReading>>({});

  /// And the buffer behind each: the text as it stands in the editor, which
  /// starts as what was read and is what the human types into.
  ///
  /// Held apart from the reading rather than written back over it, because the
  /// two are different things: the reading is what the disk said at the version
  /// it said it at, and this is what would be written over it. The save of the
  /// task after this one is the comparison between them.
  const [buffers, setBuffers] = createSignal<Record<string, string>>({});

  /// And what each tab that is standing rather than running says: the shell
  /// ended at once, or the refusal the server answered the open with.
  ///
  /// A tab is in here or it is not, and that is the whole of the difference
  /// between the two kinds: one with an entry keeps its grid and takes no
  /// typing, and one without is a shell somebody is working in.
  const [over, setOver] = createSignal<Record<number, string>>({});

  /// What each tab's shell has called itself, where it has called itself
  /// anything: the title it last set, which is what the tab is labelled by.
  ///
  /// Kept here rather than in the window that heard it, because a name is a
  /// thing about the tab bar and the window drawing the grid may be hidden. A
  /// tab that has gone may leave an entry behind it, which names nothing: the
  /// server never reuses a number, so nothing can ever come back under it.
  const [titles, setTitles] = createSignal<Record<number, string>>({});

  /// Which tab the human turned to, where they have turned to one — by its key,
  /// that being the one word that names either kind.
  const [chosen, setChosen] = createSignal<string | undefined>();

  /// And which one they are being asked about, where a close was refused for a
  /// shell somebody is working in.
  ///
  /// The tab's number rather than a flag, because the card names the tab: a
  /// pane with several shells in it is a card that has to say which of them is
  /// about to end. Nothing while there is nothing to ask.
  const [asking, setAsking] = createSignal<number | undefined>();

  /// Whether the list has been read, which is what says the pane knows how many
  /// terminals there are. Before it, a pane with no tabs is one that has not
  /// looked yet rather than a Conversation with no shells — so the hint waits on
  /// this, a sentence about an empty pane being a thing to say once somebody has
  /// looked.
  const [read, setRead] = createSignal(false);

  /// When this pane asked for each terminal it opened, which is what
  /// [`AT_ONCE`] is measured from. Nothing for the ones that were already live:
  /// this pane never asked for those, so a shell of theirs that ends is one that
  /// ran.
  const askedAt = new Map<number, number>();

  /// Whether an open is in flight, so that a second press while the first is
  /// still being answered does not open two shells.
  let opening = false;

  /// The key the next tab standing on a refusal gets. Below every number the
  /// server issues, counting the other way, because a refused open was never
  /// given one — there is no shell for it to name.
  let refusals = 0;

  /// The one showing: the tab turned to, or the first while nobody has turned
  /// to one — and the first again once the one turned to is gone, which is what
  /// keeps the pane showing a terminal rather than a gap where one was.
  const showing = createMemo((): string | undefined => {
    const open = tabs();
    const turnedTo = chosen();

    if (turnedTo !== undefined && open.some((one) => keyed(one) === turnedTo)) {
      return turnedTo;
    }

    const first = open[0];

    return first === undefined ? undefined : keyed(first);
  });

  /// What a tab is called. What its shell last called itself, where it has
  /// called itself anything at all: a title of nothing but spaces is a shell
  /// clearing its name rather than setting a blank one, and reads as none.
  ///
  /// Failing that, the number the server issued it — and the bare word for one
  /// standing on an open that was refused, the server never having got as far as
  /// a number for that one, where a made-up one would be a name for a shell that
  /// is not there.
  const called = (tab: Tab): string => {
    if ("file" in tab) {
      return named(tab.file);
    }

    const said = titles()[tab.terminal]?.trim();

    if (said) {
      return said;
    }

    return tab.terminal > 0 ? `Terminal ${tab.terminal}` : "Terminal";
  };

  /// Take away whatever is only standing there to say why, which is what **New
  /// terminal** replacing one means: a tab that is a sentence about a shell that
  /// never started is not something to keep beside a shell that has.
  const replace = (): void => {
    const standing = Object.keys(over()).map(Number);

    if (standing.length === 0) {
      return;
    }

    setOver({});
    setTabs((was) =>
      was.filter((one) => !("terminal" in one && standing.includes(one.terminal))),
    );
  };

  /// A tab that says why there is no shell in it, which stands until somebody
  /// asks for another.
  ///
  /// A sentence rather than nothing at all: a pane that simply went back to
  /// empty would say the same thing about a Sandbox that will not start as it
  /// says about a Conversation nobody has opened a shell in.
  const stand = (why: string): void => {
    replace();

    refusals -= 1;
    const tab = refusals;

    setTabs((was) => [...was, { terminal: tab }]);
    setOver((was) => ({ ...was, [tab]: why }));
    setChosen(keyed({ terminal: tab }));
  };

  /// Open another, and show it. What **New terminal** does, from the plus at the
  /// end of the strip or from the press in the empty pane — the only two things
  /// that ask for a shell, the pane asking for none of its own accord.
  const open = (): Promise<void> => {
    if (opening) {
      return Promise.resolve();
    }

    opening = true;

    return openTerminal(props.conversation.id)
      .then((outcome) => {
        if (typeof outcome === "string") {
          stand(TERMINAL_REFUSAL[outcome]);
          return;
        }

        const { number } = outcome.Opened;

        replace();
        askedAt.set(number, Date.now());
        setTabs((was) => [...was, { terminal: number }]);
        setChosen(keyed({ terminal: number }));
      })
      // A request that never landed is a shell that did not start, and it is
      // read as one: the pane says so in a tab and waits to be asked again,
      // rather than asking again itself against a server that is not there.
      .catch((error: Error) => stand(error.message))
      .finally(() => {
        opening = false;
      });
  };

  /// One tab's socket closed, which is its shell gone: the server takes a
  /// terminal off its register the moment its shell exits, and every watcher's
  /// socket closes with it.
  ///
  /// What follows is the whole of the ending rule. A shell that ran goes, and
  /// where it was the last the pane stands empty behind it; one that ended
  /// inside [`AT_ONCE`] of being asked for could not start, so its tab stays
  /// saying so until somebody asks for another.
  const ended = (tab: number): void => {
    const asked = askedAt.get(tab);

    if (asked === undefined || Date.now() - asked >= AT_ONCE) {
      askedAt.delete(tab);
      setTabs((was) =>
        was.filter((one) => !("terminal" in one && one.terminal === tab)),
      );
      return;
    }

    setOver((was) => ({ ...was, [tab]: ENDED_AT_ONCE }));
  };

  /// Close one, which is what the × at the end of a tab does.
  ///
  /// The shell is ended at the server and the tab goes when its socket closes,
  /// which is how a tab hears about every shell that ends — one rule, whichever
  /// end asked for it. What this takes off first is the pane's clock: a shell
  /// somebody closed is a shell that ran, however long it ran for, and left in
  /// place the five seconds would read a tab closed straight after it opened as
  /// a shell that could not start.
  ///
  /// **Unless somebody is working in it**, which the server is the one to say:
  /// a shell whose foreground is something other than itself answers the press
  /// with *busy* and goes on running, and the card below is what asks. The
  /// press that comes back from the card says it was asked, and ends the shell
  /// whatever is in front of it (ADR 0019).
  ///
  /// Asked of the server at the press rather than read off the list the pane
  /// loaded with, which is why this is a request rather than a look at what is
  /// already here: a build started since the pane opened is a build.
  ///
  /// A tab that is only standing there to say why has no shell to end and no
  /// socket to hear it on, so it simply goes. Nothing is drawn about a request
  /// that failed: the shell is the server's, and a tab still there is what says
  /// it is still running.
  ///
  /// **And a file's × is the whole of closing it**: there is nothing at the
  /// server to end, so the tab goes at the press, taking its reading and its
  /// buffer with it. Opening it again is a fresh reading of the disk, which is
  /// what expanding a folder in the tree beside it is too — nothing yet tells
  /// this page that the disk moved.
  const close = (tab: Tab): void => {
    if ("file" in tab) {
      forget(tab.file);
      setTabs((was) => was.filter((one) => keyed(one) !== keyed(tab)));
      return;
    }

    const number = tab.terminal;

    if (number < 0 || over()[number] !== undefined) {
      setOver((was) => {
        const rest = { ...was };
        delete rest[number];
        return rest;
      });
      setTabs((was) => was.filter((one) => keyed(one) !== keyed(tab)));
      return;
    }

    askedAt.delete(number);

    void end(number, false);
  };

  /// What a file's tab leaves behind when it goes: nothing.
  const forget = (path: string): void => {
    setReadings((was) => {
      const rest = { ...was };
      delete rest[path];
      return rest;
    });
    setBuffers((was) => {
      const rest = { ...was };
      delete rest[path];
      return rest;
    });
  };

  /// Open a file and show it, which is what a press in the tree does.
  ///
  /// Named apart from [`open`] above rather than overloaded on it: that one
  /// asks the server for a shell, and this one reads a path. Two verbs would be
  /// one word telling a reader nothing about which.
  ///
  /// **The same file opened twice is one buffer**, so a file already open is a
  /// tab to turn to rather than a second tab beside the first (ADR 0019, *Tabs
  /// and groups*). There is one group in this stage, so one buffer is one tab.
  ///
  /// The read is made here rather than in the tree, because what it answers
  /// belongs to the tab: the version it carries is what a save will name itself
  /// as being over, and the tree is a list of names.
  const openFile = (path: string): void => {
    const tab: Tab = { file: path };

    setChosen(keyed(tab));

    if (tabs().some((one) => keyed(one) === keyed(tab))) {
      return;
    }

    setTabs((was) => [...was, tab]);

    void readFile(props.conversation.id, path)
      .then((reading) => {
        setReadings((was) => ({ ...was, [path]: reading }));

        if (typeof reading !== "string" && "Text" in reading) {
          setBuffers((was) => ({ ...was, [path]: reading.Text.text }));
        }
      })
      // A request that never landed is a file that says why there is nothing in
      // its tab, the way a file the server refused does: the sentence is the
      // server's where there is one, and this is the sentence there is instead.
      .catch((error: Error) =>
        setReadings((was) => ({
          ...was,
          [path]: { Unreadable: { why: error.message } },
        })),
      );
  };

  /// The close itself, made once with nobody asked and again with the answer.
  ///
  /// A `Busy` back is the one thing to draw: the shell is still running, and
  /// the card goes up over the tab it was pressed on. Everything else is a
  /// terminal that has ended, and the tab goes when its socket closes like any
  /// other.
  const end = (tab: number, asked: boolean): Promise<void> =>
    closeTerminal(props.conversation.id, tab, asked)
      .then((outcome) => {
        setAsking(outcome === "Busy" ? tab : undefined);
      })
      // A request that never landed leaves the tab where it is, for the reason
      // nothing is drawn about one: the shell is the server's, and a tab still
      // there is what says it is still running.
      .catch(() => setAsking(undefined));

  /// The tabs the pane loads with: one for each terminal the server is already
  /// holding.
  ///
  /// Once, for this Conversation. The list is a reading of the register at the
  /// moment the pane opened, and everything that happens to it after that
  /// happens here — a second seeding would put back a tab whose shell has since
  /// ended.
  ///
  /// The numbers alone out of what the list says: the flag beside each is what
  /// was running when the pane loaded, and what a close acts on is the reading
  /// the server takes at the press.
  let seeded: number | undefined;

  createEffect(() => {
    const live = terminals.data?.live;

    if (live === undefined || seeded === props.conversation.id) {
      return;
    }

    seeded = props.conversation.id;
    setTabs((was) => [
      ...live.map((terminal): Tab => ({ terminal: terminal.number })),
      ...was,
    ]);
    setRead(true);
  });

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Code">
          {/* The tabs beside the title, where a pane's own controls go, and the
              way to another at the end of them. Buttons that say which they are
              rather than tabs: they are all always there, `aria-pressed` is the
              one word that says which is showing, and what each one does is
              show a grid that is already drawn.

              Drawn where there are tabs. A strip holding nothing but its own
              plus says nothing the empty state under it does not say in
              words. */}
          <Show when={tabs().length > 0}>
            <div
              class={styles.tabs}
              role="group"
              aria-label="What is open in this conversation"
            >
              <For each={tabs()}>
                {(tab) => (
                  // Two buttons rather than one: the tab is pressed to show
                  // what it holds and the × is pressed to close it, and a
                  // button inside a button is not a thing a browser draws. The
                  // frame around them is the tab as the eye reads it, and is
                  // what takes the fill of the one showing.
                  <div class={styles.tabFrame}>
                    <button
                      type="button"
                      class={styles.tab}
                      aria-pressed={showing() === keyed(tab)}
                      onClick={() => setChosen(keyed(tab))}
                    >
                      {/* What kind of thing the tab holds, which is the one
                          thing an icon at that end says: a shell, or a
                          file. */}
                      <Icon
                        of={"file" in tab ? faFile : faTerminal}
                        class={styles.kind}
                      />
                      <span class={styles.name}>{called(tab)}</span>
                    </button>

                    {/* And the way to end it. Called by the tab it would close:
                        an icon says nothing when it is read aloud, and a row of
                        these all saying "Close" would say nothing about
                        which. */}
                    <button
                      type="button"
                      class={styles.close}
                      aria-label={`Close ${called(tab)}`}
                      onClick={() => close(tab)}
                    >
                      <Icon of={faXmark} />
                    </button>
                  </div>
                )}
              </For>

              <IconButton
                of={faPlus}
                label="New terminal"
                class={styles.plus}
                // Nothing of this one is open: it opens a shell rather than a
                // pane, and there is no state of the page it is the way back
                // into.
                open={false}
                press={() => void open()}
              />
            </div>
          </Show>
        </PaneHead>
      </PaneSticky>

      {/* The tree and the group of tabs, side by side, which is the whole of
          the pane under its header. Both names are the frame's: `paneScreen`
          is what says this is the thing the pane sizes to its own height, and
          `paneWide` is what takes the reading measure off the pane — a tree and
          a terminal are neither of them prose, and every column they are given
          is a column they use. */}
      <div class={`${styles.body} ${shell.paneScreen} ${shell.paneWide}`}>
        <Tree conversation={props.conversation.id} open={openFile} />

        <div class={styles.group}>
          <Switch
            fallback={<Empty>Reading this conversation's terminals…</Empty>}
          >
            <Match when={terminals.isError}>
              <ErrorLine>
                Could not read this conversation's terminals:{" "}
                {terminals.error?.message}
              </ErrorLine>
            </Match>
            <Match when={tabs().length > 0}>
              <For each={tabs()}>
                {(tab) =>
                  "file" in tab ? (
                    // A file is drawn while it is the one showing and not
                    // otherwise: what it holds is the reading and the buffer
                    // above, so a tab turned away from and back to finds its
                    // text where it was. A terminal cannot be drawn that way —
                    // its grid is the socket's, and a socket closed on being
                    // hidden is a shell nobody could come back to.
                    <Show when={showing() === keyed(tab)}>
                      <Opened
                        reading={readings()[tab.file]}
                        text={buffers()[tab.file]}
                        name={named(tab.file)}
                        typed={(text) =>
                          setBuffers((was) => ({ ...was, [tab.file]: text }))
                        }
                      />
                    </Show>
                  ) : tab.terminal > 0 ? (
                    <Attached
                      at={terminalSocket(props.conversation.id, tab.terminal)}
                      showing={showing() === keyed(tab)}
                      scrollback={SCROLLBACK}
                      over={over()[tab.terminal]}
                      titled={(title) =>
                        setTitles((was) => ({ ...was, [tab.terminal]: title }))
                      }
                      ended={() => ended(tab.terminal)}
                      say={{
                        waiting:
                          "Starting a shell in this conversation's worktree…",
                        lost: "The connection to this terminal was lost.",
                      }}
                    />
                  ) : (
                    // A tab the server never opened a shell for has no grid to
                    // stand under the sentence, and nothing to attach to: the
                    // refusal is the whole of it.
                    <Show when={showing() === keyed(tab)}>
                      <ErrorLine>{over()[tab.terminal]}</ErrorLine>
                    </Show>
                  )
                }
              </For>
            </Match>

            {/* And the group with nothing in it, which is where a Conversation
                with no shells running lands and where the last of them leaves
                it. Drawn where a tab's content goes rather than over the whole
                pane: the tree stands beside this, and a pane-wide notice would
                be a sentence over that too. */}
            <Match when={read()}>
              <div class={styles.nothing}>
                <Empty>{NOTHING_OPEN}</Empty>
                <QuietButton onClick={() => void open()}>
                  New terminal
                </QuietButton>
              </div>
            </Match>
          </Switch>
        </div>
      </div>

      {/* And the card a × on a busy tab puts up, which the press that made it
          is waiting on. Outside the Switch above because it is drawn over the
          page rather than in the pane: what is behind it is whichever of those
          arms the pane is in. */}
      <Busy
        asked={
          asking() === undefined ? null : called({ terminal: asking()! })
        }
        keep={() => setAsking(undefined)}
        close={() => {
          const tab = asking();

          setAsking(undefined);

          if (tab !== undefined) {
            void end(tab, true);
          }
        }}
      />
    </>
  );
}

/// What a × on a tab whose shell has something running in it is answered with,
/// before anything at all has happened: which shell it is, what is true of it,
/// and the two ways out.
///
/// The Actions menu's own confirm card, asked about a shell instead of a run —
/// the same modal, the same pair of presses, the same shape — because it is the
/// same question: a press that would end something somebody is in the middle
/// of, put back to them before it is made (ADR 0019, *Tabs and groups*).
///
/// The tab's name in the confirming press for the reason the menu's carries the
/// pressed row's: a pane with four shells in it is a card that has to say which
/// of them this was.
function Busy(props: {
  /// The tab's name, or `null` while nothing is being asked about.
  asked: string | null;
  /// The way back, which Escape and a press on the backdrop come to as well:
  /// every way out of this card but the one button leaves the shell running.
  keep: () => void;
  /// And the press it asked about, made — which ends the shell whatever is in
  /// front of it.
  close: () => void;
}): JSX.Element {
  // Generated rather than written, the way every other card in the app names
  // itself: more than one pane stands on a page at once.
  const id = createUniqueId();

  return (
    <Modal
      class={styles.confirming!}
      open={props.asked !== null}
      close={props.keep}
      labelledBy={id}
    >
      <p id={id} class={styles.confirmingTitle}>
        Close this terminal while something is running?
      </p>
      <p class={styles.confirmingWhy}>
        Something other than the shell is running in {props.asked}. Closing the
        tab ends the shell, and whatever it is running goes with it.
      </p>
      <div class={styles.confirmingOut}>
        {/* Both classes, as every other confirm pair in the app carries them:
            the global one is the paint, and the module's is what the row above
            stands the filled press out of. */}
        <button
          type="button"
          class={`${styles.secondary!} secondary`}
          onClick={() => props.keep()}
        >
          Keep it running
        </button>
        <button type="button" onClick={() => props.close()}>
          Close {props.asked}
        </button>
      </div>
    </Modal>
  );
}

/// What is in a file's tab: the text in an editor, the picture, or the line
/// saying why there is nothing to draw.
///
/// The four kinds a read can come back as, and the two ways one can come back
/// saying nothing (ADR 0019, *Monaco, whole*). Which it is is the server's
/// reading of the bytes rather than a guess made from the name — the bytes are
/// the server's, and the whole point of the last two kinds is that they never
/// cross the wire.
///
/// **The editor is [`./Editor`]**, which is Monaco. What is kept here is what
/// is around it: which of the four kinds came back, and the buffer the text is
/// in — so the editor is handed a file and its text and has nothing else to
/// know.
function Opened(props: {
  /// What came back, or nothing at all while the read is in flight.
  reading: FileReading | undefined;
  /// The buffer: the text as it stands, which starts as what was read.
  text: string | undefined;
  /// What the file is called, which is what an editor and a picture alike are
  /// read aloud as.
  name: string;
  /// And what typing into it does.
  typed: (text: string) => void;
}): JSX.Element {
  /// The refusal this came back as, where it came back as one — the server's
  /// own sentence for the unreadable, which is the only one of them that says
  /// something this side could not have worked out.
  const why = (): string | null => {
    const read = props.reading;

    if (read === undefined) {
      return null;
    }

    if (typeof read !== "string") {
      return "Unreadable" in read ? read.Unreadable.why : null;
    }

    return FILE_REFUSAL[read];
  };

  const text = (): Extract<FileReading, { Text: unknown }>["Text"] | null => {
    const read = props.reading;

    return read !== undefined && typeof read !== "string" && "Text" in read
      ? read.Text
      : null;
  };

  const image = (): Extract<FileReading, { Image: unknown }>["Image"] | null => {
    const read = props.reading;

    return read !== undefined && typeof read !== "string" && "Image" in read
      ? read.Image
      : null;
  };

  return (
    <Switch fallback={<Empty>Opening this file…</Empty>}>
      <Match when={why()}>{(said) => <ErrorLine>{said()}</ErrorLine>}</Match>
      <Match when={text()}>
        {(read) => (
          // Monaco, coloured by the path it was read at — and a file in a
          // read-only root is an editor that takes no typing, the root's own
          // flag rather than the file's mode, which is what saves a human
          // finding out by typing.
          <Editor
            path={read().path}
            name={props.name}
            text={props.text ?? read().text}
            writable={read().writable}
            typed={props.typed}
          />
        )}
      </Match>
      <Match when={image()}>
        {(drawn) => (
          // The bytes came with the reading rather than through a second
          // request, so the picture is drawn out of what is already here.
          <div class={styles.picture}>
            <img
              src={`data:${drawn().media_type};base64,${drawn().base64}`}
              alt={props.name}
            />
          </div>
        )}
      </Match>
    </Switch>
  );
}
