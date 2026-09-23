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
//! **A tree down the side and groups of tabs beside it**, which is the shape
//! of the whole pane. The tree is [`./Tree`]: a root per Worktree the
//! Conversation has, one folder read when it is expanded. The groups are what
//! is in this file, and each of them holds files and terminals alike — one bar,
//! one set of tabs, because a shell and a file are two things to have open
//! rather than two kinds of pane (ADR 0019, *Tabs and groups*).
//!
//! **And how they stand is a tree** — see [`./layout`]. A group splits to the
//! right of itself or below itself, either half splits again to any depth, and
//! what holds them is split nodes with a share per child and groups at the
//! leaves. It is the one thing this pane is drawn from: what is showing, where
//! the borders between the groups are, and which group is **active** are all
//! read off it. The shares are percentages of the parent for the reason the
//! frame's pane widths are percentages of the window — a border settled on a
//! laptop should mean the same on a wider screen (see `widths.ts`).
//!
//! **Two ways to make one, and a third later in this stage.** A group splits
//! from the menu a right-click on one of its tabs drops, which names both
//! directions, and from the icon at the end of its bar, which is the common
//! one — VS Code's own bar button, splitting beside. The third is a tab dragged
//! to an edge. The tab the split was made from goes on showing in both groups,
//! which since the buffers moved above this pane is one text under two views.
//!
//! **And a group whose last tab leaves disappears**, its neighbour taking the
//! room. That is the whole of unsplitting — there is no command for it (ADR
//! 0019) — so a nested group emptied collapses the split it stood in and the
//! tree shrinks a level. The last group is the exception and stays, empty: a
//! pane with nothing open is one group saying so.
//!
//! **The active group is the one last pressed into**, wherever in it the press
//! landed, and it is where an opening goes: a file pressed in the tree, a
//! terminal asked for, the tab standing on a shell that would not start. A
//! split's own new group is active, that being where the work was going.
//!
//! **A file pressed in the tree opens as a tab of the active group.** What the
//! server answers a read with says which of four kinds of thing it read — text,
//! an image, a binary it will not send, or a file over the size cap — and the
//! tab draws each as what it is: the text in an editor, the picture in its tab,
//! and either of the last two as the line saying why there is nothing to draw.
//! A file in a read-only root opens read-only and takes no typing, which is the
//! root's own flag rather than the file's mode.
//!
//! **The editor is Monaco**, whole, in [`./Editor`]: every built-in language
//! coloured and the four bundled services answering, fetched as a chunk of its
//! own the first time this pane opens and never before (ADR 0019, *Monaco,
//! whole*). This pane's part in that is the `load` below, which is the fetch
//! starting because Code was opened rather than because a file was pressed.
//!
//! The buffer is what the human's text is in and the reading is what the disk
//! said — two things rather than one, because the whole of saving is a
//! comparison between them over the version the read carried.
//!
//! **And the buffer is held above this pane**, with the tabs and the readings,
//! rather than inside the editor drawing it — see [`./keeping`]. A buffer is a
//! Monaco model registered at the file's own address and the package refuses a
//! second one there, so one made by an editor could never be shared with a
//! second editor of the same file: holding it above is what lets the same file
//! be open in two groups over one text, one undo stack and one dot (ADR 0019,
//! *Tabs and groups*). Everything here that reads or writes the text — the dot,
//! Ctrl+S, **Reload** and **Keep mine** — goes through it.
//!
//! **Saving is explicit**, which is Ctrl+S: a dot on a tab whose buffer has
//! come apart from its reading, a write that names the version that reading
//! carried, and a confirm on closing a tab still wearing the dot. VS Code's
//! own default, and the one the bar below depends on — an autosave has no
//! dirty state to hold the human's text in while they decide (ADR 0019,
//! *Versioned reads, and a stale write is refused*).
//!
//! **And a write over a file the agent has changed since is refused**, which
//! is the point of the version rather than a limit of it: last writer wins
//! would be an agent's edit silently overwritten by a human who never saw it.
//! What the refusal draws is the bar in [`Moved`] — **Reload**, which takes the
//! disk's text and the version with it, and **Keep mine**, which keeps the
//! human's text over the version the disk now has so that their next save
//! lands. Both are the same read of the file, differing only in what becomes of
//! the text in the editor, and neither writes anything: the bar is about which
//! text the *next* save is of. Stage 04 of the roadmap puts the same bar up the
//! moment the disk moves rather than at the next save; here it is drawn by the
//! refusal.
//!
//! **And the same file opened twice is one buffer.** Pressing a file already
//! open in the active group turns to its tab rather than opening a second
//! beside it; pressed while it is open in another group, it opens here as well
//! — two tabs, one buffer, and two views of the one text. Typing in either
//! shows in the other, the dot is on both tabs and one save clears both.
//!
//! Opened by the code icon on the Timeline's header — see `Timeline.tsx` —
//! which is a details pane like every other, at a path of its own so it survives
//! a reload and can be linked to. The second pane nothing on the record opens: a
//! terminal belongs to the Conversation rather than to any moment on it, the way
//! sharing does.
//!
//! **The Screen's own viewer, filling the group it is in.** What is drawn in a
//! terminal tab is [`./Attached`], the same xterm over the same socket to the
//! same server-held virtual terminal a session's Screen is watched through —
//! what runs on this one is a shell rather than an agent, and that is the whole
//! of the difference. The pane gives the pair every inch it has: the reading
//! measure every other details pane pads its content to comes off, the way the
//! composer takes it off, and the pane ends where the window does, so the tree
//! and the terminal are sized to the pane rather than scrolling it.
//!
//! **Several of them, one per tab.** A group's bar holds a tab per thing open
//! in it, in the order they were opened, with the split and then a plus that
//! opens another terminal at the end. It is the Output pane's Transcript/Screen
//! switch built again — pressed-or-not buttons in a group rather than a
//! tablist, which is the house's answer to this shape — restyled after VS
//! Code's bar, which is what the pane is drawn after from here on: a kind icon
//! at one end of every tab and a × at the other, and the tabs abutting rather
//! than spaced. The kind is the whole of what an icon there says, and there are
//! two of them: a shell, and a file.
//!
//! A bar is drawn where its group has tabs to draw. A strip holding nothing but
//! its own plus is furniture about tabs that are not there, and a pane with
//! nothing open has the hint under it to say the same thing in words — which is
//! only ever the last group, every other one going the moment its last tab
//! does.
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
//! **A file's tab is hidden rather than taken down too**, for the near reason:
//! an editor taken down is a caret and an undo stack nobody can come back to.
//! The text was never what was at risk — the buffer is above this pane — so
//! what the hiding keeps is where in the file the human was.
//!
//! **The server holds the shells, so this pane is the way back to them rather
//! than where they live.** On load it asks which of the Conversation's terminals
//! are live and draws a tab for each — so a reload, a second device or a tab
//! closed by accident comes back to what was already there, still running and
//! showing what it last showed.
//!
//! **And what is open outlives the pane.** The details pane draws one thing at
//! a time and takes down whatever it is not showing, so opening an Event and
//! coming back is a fresh mount of this file — which would be an empty pane
//! over tabs somebody had opened and text they had typed and not saved, beside
//! a tree back at its roots however far down they had walked. So
//! none of that is held here: it is held above the frame's switch, per
//! Conversation, and handed in (see [`./keeping`], and `Workbench.tsx` where it
//! is kept). The device's storage that carries the same thing through a
//! *reload* is stage 02 of the roadmap; what stands here survives the swap, and
//! a page left with text nobody has saved warns on the way out, the browser's
//! own way.
//!
//! The terminals are the exception, and the register is why: a shell is the
//! server's, so the list is read on every opening and the tabs are settled
//! against it. That settling walks the tree rather than a list: one whose shell
//! has ended since is dropped from whichever group holds it — and the group
//! with it, where it was that group's last tab — and one the register has that
//! no group does joins the active group, which is how a shell opened on another
//! device arrives.
//!
//! **And a maximise toggle at the end of the header**, which gives the editor
//! the window: the sidebar and the Timeline go, and this pane takes what they
//! were standing in. Off when Code first opens and remembered per device beside
//! the pane widths, and drawn only where those panes are up — below that
//! breakpoint the window is walked one pane at a time and this one already has
//! it (ADR 0019, *One pane, replacing the Terminal*). What it hides belongs to
//! the frame, so the hiding is done there: this pane draws the press and is
//! handed the state — see `Workbench.tsx`.
//!
//! **And a tab is closed by the × at its end** — a file's as much as a shell's,
//! the file's taking its reading and its buffer with it where it was that
//! file's last view, so that opening it again is a fresh reading of the disk
//! the way expanding a folder is. A second group still showing it is the one
//! buffer still being read, and what closes there is a view rather than a file.
//! ADR 0013 kept Close on a context menu, a × beside a label this small being a
//! thing to hit by accident and what it would end a shell somebody is working
//! in; ADR 0019 puts it on the tab, because the × is what VS Code's bar has,
//! what carries that worry now is the confirm a *busy* shell asks for, and a
//! long press with no menu on the mouse's side of it is what frees the gesture
//! for dragging a tab. The press asks the server to end that shell, and the tab
//! then goes the way every ended shell's tab goes: its socket closes, and the
//! tab closes with it, in every group that was showing it.
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
  faCompress,
  faExpand,
  faFile,
  faPlus,
  faTableColumns,
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
import { ContextMenu } from "../Menu";
import { Modal } from "../Modal";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
import {
  closeTerminal,
  listTerminals,
  openTerminal,
  readFile,
  terminalSocket,
  writeFile,
} from "../api/client";
import type {
  ConversationView,
  FileReading,
  FileWritten,
  TerminalOpened,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Attached } from "./Attached";
import { Editor } from "./Editor";
import { load } from "./editing";
import {
  dirty as unsavedIn,
  disk as onDisk,
  type Bar,
  type Buffer,
  type Kept,
  type Tab,
} from "./keeping";
import {
  found,
  groups as groupsOf,
  neighbour,
  placed,
  split,
  without,
  type Group,
  type Way,
} from "./layout";
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
/// are into a group: a file out of the tree beside it, and a shell of the
/// human's own in the Worktree. The press under it opens the second, the first
/// being a press on something already drawn.
///
/// Only ever the last group says it. Every other group goes the moment its last
/// tab does, and one that is the whole of the pane has nowhere to go.
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

/// Each way a save can be refused, in the words of what it is.
///
/// [`FILE_REFUSAL`]'s siblings, said about a write — the bound is one bound,
/// and a file is written no more widely than it is read — with the one that is
/// a write's alone at the head of them: a root that takes no writes. Each its
/// own sentence for the reason every list in this pane has one apiece.
///
/// `Stale` is excluded rather than left out, and by the type: a save refused
/// for a file that has moved is the bar below, which is a question rather than
/// a sentence — see [`Moved`]. So a refusal added to the wire later has to be
/// worded here, and that one has to go on not being.
export const WRITE_REFUSAL: Record<
  Exclude<Extract<FileWritten, string>, "Stale">,
  string
> = {
  ReadOnly:
    "This conversation only reads that worktree, so nothing in it can be saved.",
  Outside: "That is not in any of this conversation's worktrees.",
  UnderGit: "Code does not write inside a repository's .git.",
  RootGone: "This worktree is no longer on disk, so there is nowhere to save.",
  Missing: "That file is no longer there, so there was nothing to save over.",
  NotAFile: "That is a folder rather than a file.",
};

/// And what the bar over a refused stale save says.
///
/// The collision itself, in the words of what happened rather than of what went
/// wrong: the agent writes the same Worktree, and a file it rewrote while
/// somebody had it open is the ordinary way of things rather than a fault. What
/// the two presses under it do is named in them (ADR 0019, *Versioned reads,
/// and a stale write is refused*).
export const MOVED =
  "This file changed on disk while you were editing it, so nothing was saved.";

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

  /// Everything this Conversation has open, which is held above this pane
  /// rather than in it — see [`./keeping`].
  ///
  /// The frame draws one details pane at a time and takes down whatever it is
  /// not showing, so a pane swapped for an Event and back is a fresh mount: what
  /// was in this pane's own signals would be gone, tabs and unsaved text
  /// together. Handed in instead, and the swap does not reach it.
  ///
  /// One Conversation's, which is the one being drawn: the pane is mounted
  /// afresh when the Conversation changes, so this is the same keeping for as
  /// long as it stands.
  held: Kept;

  /// The maximise toggle's state and the way to change it, where the window is
  /// wide enough to have something to hide.
  ///
  /// Absent below that breakpoint, which is what takes the toggle out of the
  /// header: the details pane already has the window there, and a press that
  /// hid nothing would be a control about a layout that is not standing (ADR
  /// 0019, *One pane, replacing the Terminal*). What it hides is the frame's,
  /// so what it is remembered in is the frame's too — the toggle is here
  /// because the pane it gives the window to is.
  maximise?: { on: boolean; set: (on: boolean) => void };
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

  /// Everything the pane holds, which is the keeping it was handed: how the
  /// pane is divided and which group was last pressed into, what each file was
  /// read as, the buffers the human is typing into, which folders of the tree
  /// beside them are open, the sentences over the tabs standing on a shell that
  /// never started, the names the shells have given themselves, what each save
  /// came to, which saves are in flight, and when each shell this pane opened
  /// was asked for.
  ///
  /// Taken apart once rather than reached through the prop at every use: the
  /// pane is mounted afresh when the Conversation changes, so what is here is
  /// the one keeping for the whole of its life. Everything below reads and
  /// writes these exactly as it did when they were the pane's own — which is
  /// the point of handing them over whole.
  const {
    layout,
    setLayout,
    active,
    setActive,
    group: fresh,
    readings,
    setReadings,
    buffers,
    hold,
    release,
    expanded,
    setExpanded,
    over,
    setOver,
    titles,
    setTitles,
    bars,
    setBars,
    saving,
    askedAt,
    refuse,
  } = props.held;

  /// And which one they are being asked about, where a close was refused for a
  /// shell somebody is working in.
  ///
  /// The tab's number rather than a flag, because the card names the tab: a
  /// pane with several shells in it is a card that has to say which of them is
  /// about to end. Nothing while there is nothing to ask.
  const [asking, setAsking] = createSignal<number | undefined>();

  /// And which view of which file they are being asked about, where a × was
  /// pressed on a tab holding text nobody has saved.
  ///
  /// The path rather than a flag, for the reason the number beside it is one: a
  /// pane with several files open is a card that has to say which of them is
  /// about to lose its text. And the group with it, because the press that
  /// comes back from the card closes the tab it was made on rather than every
  /// view of that file.
  const [leaving, setLeaving] = createSignal<
    { group: Group; path: string } | undefined
  >();

  /// And which tab was right-clicked, where a tab's own menu is open: where the
  /// pointer was, and the group and the tab it was over.
  ///
  /// The whole of what the menu is about — a split is made *from* a tab, of the
  /// group that tab is in — and `null` while there is nothing open.
  const [pointed, setPointed] = createSignal<{
    at: { x: number; y: number };
    group: Group;
    tab: Tab;
  } | null>(null);

  /// Whether the gesture that is opening a menu began under a finger.
  ///
  /// A phone has no right-click and fires `contextmenu` from a long press,
  /// which is the gesture a tab is dragged with (ADR 0019, *Tabs and groups*).
  /// So the menu is the mouse's alone, and what tells the two apart is the
  /// pointer that started the press rather than the event itself, which carries
  /// nothing about the hand that made it — the sidebar's cards say the same
  /// thing the same way, in `Conversations.tsx`.
  let fromTouch = false;

  /// Whether the list has been read, which is what says the pane knows how many
  /// terminals there are. Before it, a pane with no tabs is one that has not
  /// looked yet rather than a Conversation with no shells — so the hint waits on
  /// this, a sentence about an empty pane being a thing to say once somebody has
  /// looked.
  const [read, setRead] = createSignal(false);

  /// Whether an open is in flight, so that a second press while the first is
  /// still being answered does not open two shells.
  ///
  /// The pane's own rather than the keeping's, as the two cards below are: it
  /// guards a request this mount made, and a pane taken down while one was in
  /// flight has no press left to guard.
  let opening = false;

  /// Every group there is, in the order the pane draws them, and where each of
  /// them stands — both read off the tree, which is the one thing the pane is
  /// drawn from (see [`./layout`]).
  ///
  /// The groups are objects that outlive the tree they are in: a tab opened,
  /// turned to or closed writes a signal inside one rather than making a new
  /// tree, so `For` below reconciles these by identity and nothing is taken
  /// down and made again for a press in the group beside it.
  const groups = createMemo(() => groupsOf(layout()));
  const placing = createMemo(() => placed(layout()));

  /// The group last pressed into, which is where an opening lands: a file
  /// pressed in the tree, a terminal asked for, the tab standing on a shell
  /// that would not start.
  ///
  /// The first group where the active one has gone, which is what a layout in
  /// the middle of collapsing looks like for the length of one press.
  const into = (): Group => found(layout(), active()) ?? groups()[0]!;

  /// Where a group stands, as the percentages the tree works out: the whole of
  /// how the pane is divided, said to the browser.
  const stood = (id: number): JSX.CSSProperties => {
    const at = placing()[id];

    return at === undefined
      ? {}
      : {
          left: `${at.x}%`,
          top: `${at.y}%`,
          width: `${at.width}%`,
          height: `${at.height}%`,
        };
  };

  /// The one showing in a group: the tab turned to, or the first while nobody
  /// has turned to one — and the first again once the one turned to is gone,
  /// which is what keeps a group showing a terminal rather than a gap where one
  /// was.
  const showing = (group: Group): string | undefined => {
    const open = group.tabs();
    const turnedTo = group.chosen();

    if (turnedTo !== undefined && open.some((one) => keyed(one) === turnedTo)) {
      return turnedTo;
    }

    const first = open[0];

    return first === undefined ? undefined : keyed(first);
  };

  /// How many groups are showing a file, which is how many views its one buffer
  /// has: what says whether a tab closing is the last of them.
  const views = (path: string): number =>
    groups().filter((one) =>
      one.tabs().some((tab) => keyed(tab) === keyed({ file: path })),
    ).length;

  /// Take a group out of the layout, which is what its last tab leaving does.
  ///
  /// The split it stood in collapses and its neighbour takes the room — the
  /// whole of unsplitting, there being no command for it (ADR 0019, *Tabs and
  /// groups*). The last group is left where it is: there is always a group, and
  /// a pane with nothing open is one empty group saying so.
  ///
  /// Where the group that went was the active one, the neighbour that took its
  /// room becomes active: that is where the eye already is, and something has
  /// to be.
  const shut = (group: Group): void => {
    if (groups().length < 2) {
      return;
    }

    const next = neighbour(layout(), group.id);

    setLayout((was) => without(was, group.id));

    if (active() === group.id && next !== undefined) {
      setActive(next.id);
    }
  };

  /// A tab out of one group, and the group with it where that was its last.
  const take = (group: Group, tab: Tab): void => {
    group.setTabs((was) => was.filter((one) => keyed(one) !== keyed(tab)));

    if (group.tabs().length === 0) {
      shut(group);
    }
  };

  /// The same, of every group at once — what a shell ending does, there being
  /// no such thing as a terminal that ended in one group and not another.
  const everywhere = (change: (was: Tab[]) => Tab[]): void => {
    const drawn = groups();

    for (const group of drawn) {
      group.setTabs(change);
    }

    for (const group of drawn) {
      if (group.tabs().length === 0 && found(layout(), group.id) !== undefined) {
        shut(group);
      }
    }
  };

  /// A right-click on a tab asks which way to split the group it is in, which
  /// is the second of the two ways there are to make one — the first being the
  /// icon at the end of the bar, and the third a tab dragged to an edge, which
  /// is later in this stage.
  ///
  /// The browser's own menu is not what the hand is asking for, so that goes.
  /// A mouse's gesture and only a mouse's: a phone fires the same event from a
  /// long press, and a long press on a tab is how it will be picked up to be
  /// dragged (ADR 0019, *Tabs and groups*).
  const ask = (event: MouseEvent, group: Group, tab: Tab): void => {
    if (fromTouch) {
      return;
    }

    event.preventDefault();
    setActive(group.id);
    setPointed({ at: { x: event.clientX, y: event.clientY }, group, tab });
  };

  /// Split a group, with the tab the split was made from showing in both.
  ///
  /// As VS Code's split does, and since the buffers moved above the pane that
  /// is one text under two views: typing in either shows in the other, the dot
  /// is on both tabs and one save clears both (ADR 0019, *Tabs and groups*).
  ///
  /// The new group is the active one. It is where the split was asked for and
  /// where the work is going, so it is where the next file pressed in the tree
  /// should open.
  const divide = (group: Group, tab: Tab, way: Way): void => {
    const made = fresh();

    made.setTabs([tab]);
    made.setChosen(keyed(tab));

    setLayout((was) => split(was, group.id, way, made));
    setActive(made.id);
  };

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
    everywhere((was) =>
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

    const tab = refuse();
    const group = into();

    group.setTabs((was) => [...was, { terminal: tab }]);
    setOver((was) => ({ ...was, [tab]: why }));
    group.setChosen(keyed({ terminal: tab }));
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

        const group = into();

        group.setTabs((was) => [...was, { terminal: number }]);
        group.setChosen(keyed({ terminal: number }));
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
      everywhere((was) =>
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
  ///
  /// **Unless there is text in it nobody has saved**, which is the other half
  /// of the rule a busy shell's confirm is the first of (ADR 0019, *Tabs and
  /// groups*): the press asks first, and only the press inside the card throws
  /// the text away. Asked of this page rather than of the server, unlike the
  /// shell's — what is unsaved is the buffer here, and the server has never
  /// heard of it.
  const close = (group: Group, tab: Tab): void => {
    if ("file" in tab) {
      // Only where this is the file's last view. A second group showing the
      // same file is the same buffer, so a tab closed while it stands loses
      // nothing at all and there is nothing to ask about.
      if (views(tab.file) === 1 && dirty(tab.file)) {
        setLeaving({ group, path: tab.file });
        return;
      }

      drop(group, tab.file);
      return;
    }

    const number = tab.terminal;

    if (number < 0 || over()[number] !== undefined) {
      setOver((was) => {
        const rest = { ...was };
        delete rest[number];
        return rest;
      });
      take(group, tab);
      return;
    }

    askedAt.delete(number);

    void end(number, false);
  };

  /// A file's tab, gone: the tab off the bar and everything behind it forgotten.
  ///
  /// Apart from [`close`] because it is the far side of the card as well as the
  /// near side of a clean press — what the card's own button makes is this,
  /// with the human having said so.
  const drop = (group: Group, path: string): void => {
    take(group, { file: path });

    // And what is behind it, where that tab was the last view of the file. A
    // second group still showing it is the one buffer still being read, and
    // forgetting it here would take the text out from under that view.
    if (views(path) === 0) {
      forget(path);
    }
  };

  /// What a file's last view leaves behind when it goes: nothing.
  ///
  /// The buffer with it, which is a disposal rather than a forgetting: the
  /// model is registered at the file's own address in Monaco's own register,
  /// and one left there is a file that could never be opened again.
  const forget = (path: string): void => {
    saving.delete(path);
    setReadings((was) => {
      const rest = { ...was };
      delete rest[path];
      return rest;
    });
    release(path);
    unbar(path);
  };

  /// What the disk said about one open file, and whether there is text in it
  /// that the disk has not got.
  ///
  /// Both asked of the keeping rather than worked out here, because the warning
  /// on the way out of the page asks the second of them too — of every
  /// Conversation at once, from above this pane — and one comparison written in
  /// two places would be two ideas of what unsaved means.
  const disk = (path: string) => onDisk(props.held, path);
  const dirty = (path: string): boolean => unsavedIn(props.held, path);

  /// Read a file, and put what came back where the tab draws it from.
  ///
  /// The one way this pane ever learns what is on the disk, and all three ways
  /// into a tab come through it: opening one, **Reload**, and **Keep mine**.
  /// The last two are this same read and differ in one thing — what becomes of
  /// the text in the editor — which is what `keeping` says.
  ///
  /// **Keep mine reads too**, rather than taking a version off the refusal and
  /// leaving the reading where it was. What a reading is *for* here is the
  /// comparison [`dirty`] makes, and a reading whose text the disk no longer
  /// holds would make that comparison a lie: a human who typed their way back
  /// to the text the collision was against would be shown a clean tab over a
  /// file that says something else. So the disk is asked, which is one request
  /// on a press somebody made on purpose.
  const reread = (path: string, keeping = false): Promise<void> => {
    // Whatever the last save said goes with the reading it was about: the bar
    // is a question about the disk, and this is the disk answering.
    unbar(path);

    return readFile(props.conversation.id, path)
      .then((reading) => {
        setReadings((was) => ({ ...was, [path]: reading }));

        if (keeping) {
          return;
        }

        if (typeof reading !== "string" && "Text" in reading) {
          // The buffer, made out of what was read where the file has none yet
          // and written with it where it has — which is the whole of the
          // difference between opening a file and **Reload**.
          hold(path, reading.Text.text);
        } else {
          release(path);
        }
      })
      // A request that never landed is a file that says why there is nothing in
      // its tab, the way a file the server refused does: the sentence is the
      // server's where there is one, and this is the sentence there is instead.
      .catch((error: Error) => {
        setReadings((was) => ({
          ...was,
          [path]: { Unreadable: { why: error.message } },
        }));
      });
  };

  /// Take a file's bar down, which every reading and every save that lands
  /// does: what is in there is about the last press rather than about the file.
  const unbar = (path: string): void => {
    setBars((was) => {
      const rest = { ...was };
      delete rest[path];
      return rest;
    });
  };

  /// Save one, which is what Ctrl+S does.
  ///
  /// The write names the version the read handed over, and the server refuses
  /// it where the file has moved since (ADR 0019, *Versioned reads, and a stale
  /// write is refused*). What comes back is one of three things: the file
  /// written, with the version it now has — the reading is moved onto that text
  /// and that version, which is what takes the dot off the tab; the file moved,
  /// which puts the bar up; or a refusal, which is a line.
  ///
  /// A file with nothing to save is not written at all. Ctrl+S on a clean
  /// editor is a reflex rather than a request, and a write that changed nothing
  /// would still move the file's timestamp under every watcher there is.
  const save = (path: string): Promise<void> => {
    const read = disk(path);
    const text = buffers()[path]?.text();

    if (read === undefined || text === undefined || saving.has(path)) {
      return Promise.resolve();
    }

    if (text === read.text) {
      return Promise.resolve();
    }

    saving.add(path);

    return writeFile(props.conversation.id, path, read.version, text)
      .then((written) => {
        if (typeof written !== "string" && "Written" in written) {
          // Onto what was just put there, at the version it now has: the tab is
          // clean, and the next save is a write over this.
          settle(path, written.Written.version, text);
          return;
        }

        setBars((was) => ({
          ...was,
          [path]:
            written === "Stale"
              ? "moved"
              : {
                  why:
                    typeof written === "string"
                      ? WRITE_REFUSAL[written]
                      : written.Unwritable.why,
                },
        }));
      })
      // A request that never landed is a save that did not happen, and the tab
      // says so where it would have said any other refusal: the text is still
      // the human's, and the dot is still on the tab.
      .catch((error: Error) => {
        setBars((was) => ({ ...was, [path]: { why: error.message } }));
      })
      .finally(() => saving.delete(path));
  };

  /// Move a file's reading onto the text that has just been written to it, at
  /// the version the write answered with.
  ///
  /// What a save that landed does, and the only thing in this pane that changes
  /// a reading without reading: this side knows what is on the disk because it
  /// is what it just sent, which is why [`FileWritten`]'s `Written` carries a
  /// version alone where its `Stale` carries nothing at all.
  ///
  /// And it is what takes the dot off the tab, [`dirty`] being the comparison
  /// between this text and the buffer.
  const settle = (path: string, version: string, text: string): void => {
    setReadings((was) => {
      const read = was[path];

      if (read === undefined || typeof read === "string" || !("Text" in read)) {
        return was;
      }

      return { ...was, [path]: { Text: { ...read.Text, version, text } } };
    });
    unbar(path);
  };

  /// Open a file and show it, which is what a press in the tree does.
  ///
  /// Named apart from [`open`] above rather than overloaded on it: that one
  /// asks the server for a shell, and this one reads a path. Two verbs would be
  /// one word telling a reader nothing about which.
  ///
  /// **Into the group last pressed into**, which is what active means: the
  /// press was made on the tree rather than in any group, and the group the
  /// human was last working in is the one they meant.
  ///
  /// **The same file opened twice is one buffer**, so a file already open in
  /// *this* group is a tab to turn to rather than a second tab beside the first
  /// (ADR 0019, *Tabs and groups*). Open in another group, it opens here as
  /// well: two tabs, one buffer, two views of the one text.
  ///
  /// The read is made here rather than in the tree, because what it answers
  /// belongs to the tab: the version it carries is what a save will name itself
  /// as being over, and the tree is a list of names. Made once per file rather
  /// than once per view — a second view of a file somebody has typed into and
  /// not saved would otherwise read the disk over their text.
  const openFile = (path: string): void => {
    const tab: Tab = { file: path };
    const group = into();
    const anywhere = views(path) > 0;

    group.setChosen(keyed(tab));

    if (group.tabs().some((one) => keyed(one) === keyed(tab))) {
      return;
    }

    group.setTabs((was) => [...was, tab]);

    if (!anywhere) {
      void reread(path);
    }
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

  /// Ctrl+S — Cmd+S on a Mac — which is the whole of how a file is saved.
  ///
  /// Saving is explicit, which is VS Code's default and the one the bar over a
  /// refused save depends on: an autosave has no dirty state to hold the
  /// human's text in while they decide what to do about a collision (ADR 0019,
  /// *Versioned reads, and a stale write is refused*).
  ///
  /// On the document rather than on the editor. Monaco binds nothing to this
  /// itself, so the press arrives here whether the caret is in a file, in a
  /// terminal beside it or on the tree — and what it saves is the file showing
  /// in the **active** group, which is the group the human last pressed into
  /// and so the file they are looking at whichever of those their hands were
  /// on. Refused by the browser first, its own Save Page being nothing anybody
  /// meant.
  ///
  /// Only while this pane is mounted, which is the whole reach of the listener:
  /// Code is the only thing in this workbench with a file in it to write.
  const pressed = (event: KeyboardEvent): void => {
    if (!(event.ctrlKey || event.metaKey) || event.altKey) {
      return;
    }

    if (event.key.toLowerCase() !== "s") {
      return;
    }

    const group = into();
    const open = group.tabs().find((one) => keyed(one) === showing(group));

    if (open === undefined || !("file" in open)) {
      return;
    }

    event.preventDefault();
    void save(open.file);
  };

  document.addEventListener("keydown", pressed);
  onCleanup(() => document.removeEventListener("keydown", pressed));

  /// The tabs the pane opens with, settled against the register.
  ///
  /// The files are the keeping's and come back exactly as they were left: a
  /// path, a reading and whatever was typed into it are this page's, and a pane
  /// swapped for an Event and back finds them where they were. The terminals
  /// are not — a shell is the server's, and the register is what says which of
  /// them are still running — so every opening reads the list and meets what is
  /// held against it.
  ///
  /// Which is two things. A tab whose shell has ended since is **dropped**
  /// rather than drawn, there being nothing left to attach it to; and a shell
  /// the register has that this page has no tab for is **added** at the end,
  /// which is how a shell opened on another device arrives. A tab standing on a
  /// shell that could not start is neither: what it holds is a sentence rather
  /// than a socket, so it keeps its place until somebody asks for another
  /// terminal.
  ///
  /// Once per opening, which is what the flag is: the list is a reading of the
  /// register at the moment the pane opened, and everything that happens to it
  /// after that happens here — a second settling would drop the tab of a shell
  /// this pane had just opened.
  ///
  /// The numbers alone out of what the list says: the flag beside each is what
  /// was running when the pane loaded, and what a close acts on is the reading
  /// the server takes at the press.
  let settled = false;

  createEffect(() => {
    const live = terminals.data?.live;

    if (live === undefined || settled) {
      return;
    }

    settled = true;

    const running = new Set(live.map((terminal) => terminal.number));

    // Dropped from whichever group holds it, wherever that is — and a group
    // whose last tab was a shell that has ended goes the way any other emptied
    // group goes.
    everywhere((was) =>
      was.filter(
        (one) =>
          "file" in one ||
          running.has(one.terminal) ||
          over()[one.terminal] !== undefined,
      ),
    );

    const drawn = new Set(
      groups().flatMap((group) =>
        group.tabs().flatMap((one) => ("file" in one ? [] : [one.terminal])),
      ),
    );

    const arrived = live
      .filter((terminal) => !drawn.has(terminal.number))
      .map((terminal): Tab => ({ terminal: terminal.number }));

    // And one the register has that no group does joins the active group,
    // which is how a shell opened on another device arrives.
    if (arrived.length > 0) {
      into().setTabs((was) => [...was, ...arrived]);
    }

    setRead(true);
  });

  return (
    <>
      <PaneSticky>
        <PaneHead
          // Away while the pane has the window, which is the one frame with
          // nothing beside it to go back *to*: the Timeline is not drawn, so
          // "← Timeline" would be a press that did nothing. The way out of the
          // mode is the toggle that made it.
          back={
            props.maximise?.on ? undefined : { to: "Timeline", go: props.back }
          }
          title="Code"
        >
          {/* And the way to give the editor the window, at the end of the
              header: the sidebar and the Timeline go, and the details pane —
              which is this one — takes what they were standing in (ADR 0019,
              *One pane, replacing the Terminal*).

              Drawn only where the frame has those panes up. Below that
              breakpoint the window is walked one pane at a time and this one
              already has it, so there is nothing for the press to hide and the
              toggle is not there at all.

              Its own button rather than the app's icon button, which is a way
              *into* something and draws itself as open while you are in it:
              this is a switch with two states, and what says which it is in is
              `aria-pressed` on a press that changes it. */}
          <Show when={props.maximise}>
            {(maximise) => (
              <button
                type="button"
                class={styles.maximise}
                aria-label={
                  maximise().on
                    ? "Bring the other panes back"
                    : "Give Code the window"
                }
                aria-pressed={maximise().on}
                onClick={() => maximise().set(!maximise().on)}
              >
                <Icon of={maximise().on ? faCompress : faExpand} />
              </button>
            )}
          </Show>
        </PaneHead>
      </PaneSticky>

      {/* The tree and the groups of tabs, side by side, which is the whole of
          the pane under its header. Both names are the frame's: `paneScreen`
          is what says this is the thing the pane sizes to its own height, and
          `paneWide` is what takes the reading measure off the pane — a tree and
          a terminal are neither of them prose, and every column they are given
          is a column they use. */}
      <div class={`${styles.body} ${shell.paneScreen} ${shell.paneWide}`}>
        {/* The tree's open folders come out of the same keeping the tabs do,
            and for the same reason: a swap to an Event and back would
            otherwise be the walk down to a file made over again. */}
        <Tree
          conversation={props.conversation.id}
          open={openFile}
          held={expanded}
          setHeld={setExpanded}
        />

        <div class={styles.groups}>
          {/* The register not answering is a line *above* whatever is open
              rather than in place of it. The terminals are the server's and
              this says so when it cannot be asked; the files beside them are
              this page's own — read, typed into and not yet saved — and a tab
              nobody can reach because a different half of the pane could not
              be read would be the one failure that loses somebody's text. The
              tabs are still in their bars either way, so drawing the error
              where their content goes left them pressable and empty.

              Once for the pane rather than once per group: what could not be
              read is the Conversation's register, which no group owns a part
              of. */}
          <Show when={terminals.isError}>
            <ErrorLine>
              Could not read this conversation's terminals:{" "}
              {terminals.error?.message}
            </ErrorLine>
          </Show>

          {/* And the groups themselves, every one of them drawn in the one
              layer and placed by the tree — see [`./layout`]. Placed rather
              than nested, so that a split, a collapse or a tab moved between
              two of them moves boxes about instead of taking a group's whole
              content down and making it again: a nest of boxes would cost a
              terminal its socket and an editor its caret for a press made in
              the group beside it. */}
          <div class={styles.stack}>
            <For each={groups()}>
              {(group) => (
                <div
                  class={styles.group}
                  style={stood(group.id)}
                  // Which group a file pressed in the tree will open in, said
                  // where it can be read as well as seen — and said only where
                  // there is more than one of them, a lone group being not the
                  // current one of a set but the whole of the pane.
                  aria-current={
                    groups().length > 1 && active() === group.id
                      ? "true"
                      : undefined
                  }
                  // The active group is the one last pressed into, wherever in
                  // it the press landed: a tab, a terminal's grid, an editor.
                  // Focus counts as a press for the keyboard's sake, there
                  // being no pointer to make one with.
                  onPointerDown={() => setActive(group.id)}
                  onFocusIn={() => setActive(group.id)}
                >
                  {/* The bar of this group, and the two ways on from it at the
                      end. Buttons that say which they are rather than tabs:
                      they are all always there, `aria-pressed` is the one word
                      that says which is showing, and what each one does is show
                      something that is already drawn.

                      Drawn where there are tabs. A strip holding nothing but
                      its own plus says nothing the empty state under it does
                      not say in words — and only the last group is ever empty,
                      every other one going the moment its last tab does. */}
                  <Show when={group.tabs().length > 0}>
                    <div
                      class={styles.tabs}
                      role="group"
                      aria-label="What is open in this group"
                    >
                      <For each={group.tabs()}>
                        {(tab) => (
                          // Two buttons rather than one: the tab is pressed to
                          // show what it holds and the × is pressed to close
                          // it, and a button inside a button is not a thing a
                          // browser draws. The frame around them is the tab as
                          // the eye reads it, and is what takes the fill of the
                          // one showing.
                          <div class={styles.tabFrame}>
                            <button
                              type="button"
                              class={styles.tab}
                              aria-pressed={showing(group) === keyed(tab)}
                              // Which hand is making the gesture, for the menu
                              // below: a long press is how a tab will be picked
                              // up, so the menu is the mouse's alone.
                              onPointerDown={(event) => {
                                fromTouch = event.pointerType !== "mouse";
                              }}
                              onContextMenu={(event) => ask(event, group, tab)}
                              onClick={() => {
                                setActive(group.id);
                                group.setChosen(keyed(tab));
                              }}
                            >
                              {/* What kind of thing the tab holds, which is the
                                  one thing an icon at that end says: a shell,
                                  or a file. */}
                              <Icon
                                of={"file" in tab ? faFile : faTerminal}
                                class={styles.kind}
                              />
                              <span class={styles.name}>{called(tab)}</span>

                              {/* And the dot that says there is text in it
                                  nobody has saved — VS Code's own mark, beside
                                  the name rather than in place of the × it
                                  draws it in place of: there is no hover on a
                                  phone, and a × a finger cannot find is a tab a
                                  finger cannot close.

                                  On every view of the file, there being one
                                  buffer under them: the dot is the buffer's
                                  against the disk rather than this tab's.

                                  Empty, and read aloud off its label: what it
                                  says belongs to the tab's own name, which is
                                  what a screen reader reads when it reaches the
                                  button. */}
                              <Show when={"file" in tab && dirty(tab.file)}>
                                <span
                                  class={styles.dot}
                                  role="img"
                                  aria-label="unsaved"
                                />
                              </Show>
                            </button>

                            {/* And the way to end it. Called by the tab it
                                would close: an icon says nothing when it is
                                read aloud, and a row of these all saying
                                "Close" would say nothing about which. */}
                            <button
                              type="button"
                              class={styles.close}
                              aria-label={`Close ${called(tab)}`}
                              onClick={() => close(group, tab)}
                            >
                              <Icon of={faXmark} />
                            </button>
                          </div>
                        )}
                      </For>

                      {/* The split, at the end of the bar: this group again
                          beside itself, showing the tab that was showing (ADR
                          0019, *Tabs and groups*). One way rather than two,
                          which is VS Code's bar as well — the other is a
                          right-click on the tab, where both of them are named
                          in words. */}
                      <IconButton
                        of={faTableColumns}
                        label="Split right"
                        class={styles.divide}
                        // Nothing of this one is open either: it makes a group
                        // rather than opens a pane.
                        open={false}
                        press={() => {
                          const shown = group
                            .tabs()
                            .find((one) => keyed(one) === showing(group));

                          if (shown !== undefined) {
                            divide(group, shown, "beside");
                          }
                        }}
                      />

                      <IconButton
                        of={faPlus}
                        label="New terminal"
                        class={styles.plus}
                        // Nothing of this one is open: it opens a shell rather
                        // than a pane, and there is no state of the page it is
                        // the way back into.
                        open={false}
                        press={() => {
                          setActive(group.id);
                          void open();
                        }}
                      />
                    </div>
                  </Show>

                  <Switch
                    fallback={
                      <Empty>Reading this conversation's terminals…</Empty>
                    }
                  >
                    <Match when={group.tabs().length > 0}>
                      <For each={group.tabs()}>
                        {(tab) =>
                          "file" in tab ? (
                            // A file's tab is drawn whether or not it is the
                            // one showing, and hidden when it is not — the way
                            // a terminal's is, and for the near reason: a grid
                            // taken down is a shell nobody could come back to,
                            // and an editor taken down is a caret and an undo
                            // stack nobody can come back to. The text itself
                            // was never at risk, the buffer being above this
                            // pane; what the hiding keeps is where the human
                            // was in it.
                            //
                            // One of these per *view*: the same file in two
                            // groups is two of these over the one buffer, which
                            // is what makes them type together.
                            <Opened
                              showing={showing(group) === keyed(tab)}
                              reading={readings()[tab.file]}
                              buffer={buffers()[tab.file]}
                              name={named(tab.file)}
                              bar={bars()[tab.file]}
                              reload={() => void reread(tab.file)}
                              keep={() => void reread(tab.file, true)}
                            />
                          ) : tab.terminal > 0 ? (
                            <Attached
                              at={terminalSocket(
                                props.conversation.id,
                                tab.terminal,
                              )}
                              showing={showing(group) === keyed(tab)}
                              scrollback={SCROLLBACK}
                              over={over()[tab.terminal]}
                              titled={(title) =>
                                setTitles((was) => ({
                                  ...was,
                                  [tab.terminal]: title,
                                }))
                              }
                              ended={() => ended(tab.terminal)}
                              say={{
                                waiting:
                                  "Starting a shell in this conversation's worktree…",
                                lost: "The connection to this terminal was lost.",
                              }}
                            />
                          ) : (
                            // A tab the server never opened a shell for has no
                            // grid to stand under the sentence, and nothing to
                            // attach to: the refusal is the whole of it.
                            <Show when={showing(group) === keyed(tab)}>
                              <ErrorLine>{over()[tab.terminal]}</ErrorLine>
                            </Show>
                          )
                        }
                      </For>
                    </Match>

                    {/* And the group with nothing in it, which is where a
                        Conversation with no shells running lands and where the
                        last of them leaves it. Drawn where a tab's content goes
                        rather than over the whole pane: the tree stands beside
                        this, and a pane-wide notice would be a sentence over
                        that too.

                        A list that would not read settles it as well as one
                        that did. The hint waits on somebody having looked, and
                        a read that ended in the line above is a look that is
                        over — without this the pane would sit on *Reading this
                        conversation's terminals…* under a sentence saying it
                        could not, with no way to open one and try again. */}
                    <Match when={read() || terminals.isError}>
                      <div class={styles.nothing}>
                        <Empty>{NOTHING_OPEN}</Empty>
                        <QuietButton
                          onClick={() => {
                            setActive(group.id);
                            void open();
                          }}
                        >
                          New terminal
                        </QuietButton>
                      </div>
                    </Match>
                  </Switch>
                </div>
              )}
            </For>
          </div>
        </div>
      </div>

      {/* And what a right-click on a tab drops: the two ways to split the group
          it is in, with that tab showing in both. The pane's only menu, and the
          only place either split is named in words (ADR 0019, *Tabs and
          groups*). */}
      <ContextMenu
        class={styles.tabActions!}
        name="Tab actions"
        at={pointed()?.at ?? null}
        close={() => setPointed(null)}
      >
        {() => (
          <For
            each={
              [
                ["Split right", "beside"],
                ["Split down", "below"],
              ] as const
            }
          >
            {([says, way]) => (
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  const asked = pointed();

                  setPointed(null);

                  if (asked !== null) {
                    divide(asked.group, asked.tab, way);
                  }
                }}
              >
                {says}
              </button>
            )}
          </For>
        )}
      </ContextMenu>

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

      {/* And the one a × on a file with unsaved text in it puts up, which is
          the same question about the other kind of tab. */}
      <Unsaved
        asked={leaving() === undefined ? null : named(leaving()!.path)}
        keep={() => setLeaving(undefined)}
        close={() => {
          const view = leaving();

          setLeaving(undefined);

          if (view !== undefined) {
            drop(view.group, view.path);
          }
        }}
      />
    </>
  );
}

/// What a × on a tab holding text nobody has saved is answered with, before the
/// text is thrown away: which file it is, what is about to happen to it, and the
/// two ways out.
///
/// [`Busy`]'s card asked about a file instead of a shell — the same modal, the
/// same pair of presses, the same shape — because it is the same question, and
/// the two halves of one rule: a press that would end something somebody is in
/// the middle of, put back to them before it is made (ADR 0019, *Tabs and
/// groups*).
///
/// What is different is where the answer comes from. A busy shell is the
/// server's reading, so that card goes up on a request coming back; unsaved
/// text is this page's own, so this one goes up at the press.
function Unsaved(props: {
  /// The file's name, or `null` while nothing is being asked about.
  asked: string | null;
  /// The way back, which Escape and a press on the backdrop come to as well:
  /// every way out of this card but the one button keeps the tab and its text.
  keep: () => void;
  /// And the press it asked about, made — which closes the tab and loses what
  /// was typed into it.
  close: () => void;
}): JSX.Element {
  const id = createUniqueId();

  return (
    <Modal
      class={styles.confirming!}
      open={props.asked !== null}
      close={props.keep}
      labelledBy={id}
    >
      <p id={id} class={styles.confirmingTitle}>
        Close this file without saving it?
      </p>
      <p class={styles.confirmingWhy}>
        {props.asked} has changes that are not on disk. Closing the tab throws
        them away.
      </p>
      <div class={styles.confirmingOut}>
        <button
          type="button"
          class={`${styles.secondary!} secondary`}
          onClick={() => props.keep()}
        >
          Keep editing
        </button>
        <button type="button" onClick={() => props.close()}>
          Close {props.asked}
        </button>
      </div>
    </Modal>
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
/// in — so the editor is handed a buffer and has nothing else to know.
///
/// **Drawn hidden rather than taken down** when its tab is not the one showing,
/// which is what keeps the caret and the undo stack across a turn away and
/// back. `hidden` and a rule in the stylesheet, the way `./Attached` hides a
/// terminal's grid: the browser's own word for it, and the one thing a screen
/// reader reads the same way.
function Opened(props: {
  /// Whether this is the tab showing. Hidden rather than gone when it is not.
  showing: boolean;
  /// What came back, or nothing at all while the read is in flight.
  reading: FileReading | undefined;
  /// The buffer: the text as it stands, which starts as what was read, and is
  /// nothing at all until Monaco has landed.
  buffer: Buffer | undefined;
  /// What the file is called, which is what an editor and a picture alike are
  /// read aloud as.
  name: string;
  /// What the last save came to, where it came to anything to draw.
  bar: Bar | undefined;
  /// **Reload**: take what is on the disk now, text and version together.
  reload: () => void;
  /// And **Keep mine**: read the disk for its version, and keep the text that
  /// is here over it, so that the next save lands.
  keep: () => void;
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

  /// Whether the last save was refused for a file that has moved, which is
  /// what the bar with the two presses on it is drawn from.
  const moved = (): boolean => props.bar === "moved";

  /// And the line over one refused outright, which is the other thing a bar
  /// can be: a sentence rather than a question, there being nothing to choose
  /// between.
  const refused = (): string | null =>
    props.bar !== undefined && props.bar !== "moved" ? props.bar.why : null;

  return (
    <div class={styles.opened} hidden={!props.showing}>
      {/* Above whatever the tab is holding, because it is about the file rather
          than about the editor: a file that turned into something there is no
          editor for between the read and the save still has a save to say
          something about. */}
      <Show when={moved()}>
        <Moved reload={props.reload} keep={props.keep} />
      </Show>
      <Show when={refused()}>{(said) => <ErrorLine>{said()}</ErrorLine>}</Show>

      <Switch fallback={<Empty>Opening this file…</Empty>}>
        <Match when={why()}>{(said) => <ErrorLine>{said()}</ErrorLine>}</Match>
        <Match when={text()}>
          {(read) => (
            // Monaco over the buffer above, coloured by the path that buffer
            // was made at — and a file in a read-only root is an editor that
            // takes no typing, the root's own flag rather than the file's mode,
            // which is what saves a human finding out by typing.
            <Editor
              name={props.name}
              model={props.buffer?.model}
              writable={read().writable}
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
    </div>
  );
}

/// The bar a save refused for a file that has moved puts up: what happened, and
/// the two things to do about it.
///
/// **The human chooses**, which is the whole of why the write was refused
/// rather than made (ADR 0019, *Versioned reads, and a stale write is
/// refused*). *Reload* takes what is on the disk now — the agent's text, and
/// the version that goes with it — and the tab is clean over it. *Keep mine*
/// keeps what is in the editor and takes the disk's version alone, so that the
/// next Ctrl+S is a write over what is really there and lands.
///
/// Both of them read the file, and that is the whole of what is under either:
/// one press keeps what comes back and the other throws it away, which is why
/// neither needs a version off the refusal to work from.
///
/// Neither of them writes anything: what is on the disk is on the disk until
/// somebody saves over it, and this bar is about which text the next save will
/// be of.
///
/// Drawn in the tab rather than as a card over the page, unlike the two
/// confirms above: nothing is waiting on it — the editor below takes typing
/// while it stands — and a file whose save was refused is a thing to come back
/// to rather than a question to get out of the way. Stage 04 of the roadmap
/// puts this same bar up the moment the disk moves, rather than at the next
/// save; here it is drawn by the refusal.
export function Moved(props: {
  reload: () => void;
  keep: () => void;
}): JSX.Element {
  return (
    <div class={styles.moved} role="status">
      <p class={styles.movedWhy}>{MOVED}</p>
      <div class={styles.movedOut}>
        <QuietButton onClick={() => props.reload()}>Reload</QuietButton>
        <QuietButton onClick={() => props.keep()}>Keep mine</QuietButton>
      </div>
    </div>
  );
}
