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
//! **And every border in it is a divider that drags.** One per split, over the
//! line its two halves meet on: dragging it moves the shares either side of it
//! and nothing else in the tree, and the arrow keys nudge it by the same travel
//! along its own split's axis — left and right for a split down the middle, up
//! and down for one across. The frame's own divider is the pattern, and what is
//! different is that Code's splits go both ways. It is clamped to a floor per
//! group, which is a length rather than a share for the reason the frame's
//! minimums are: what makes a group too narrow is the bar standing in it. So
//! this pane measures the layer its groups are drawn in, the way the frame
//! measures itself.
//!
//! **Three ways to make one.** A group splits from the menu a right-click on
//! one of its tabs drops, which names both directions, and from the icon at the
//! end of its bar, which is the common one — VS Code's own bar button,
//! splitting beside. The tab the split was made from goes on showing in both
//! groups, which since the buffers moved above this pane is one text under two
//! views. The third is a tab dragged to one of a group's four edges, which
//! splits it there with that tab alone in the new group (ADR 0019, *Tabs and
//! groups*).
//!
//! **And a group whose last tab leaves disappears**, its neighbour taking the
//! room. That is the whole of unsplitting — there is no command for it (ADR
//! 0019) — so a nested group emptied collapses the split it stood in and the
//! tree shrinks a level. The last group is the exception and stays, empty: a
//! pane with nothing open is one group saying so.
//!
//! **And a tab is picked up and put down with the pointer**, the way a card in
//! the sidebar is and for the same reasons: the pointer captured at the press,
//! the rest of the gesture watched at the window so that a hand which has
//! outrun the tab is still dragging it, a grace distance before a press becomes
//! a drag, a hold rather than a distance to lift one under a finger, and every
//! way a gesture can end putting the tab down (see `Conversations.tsx`). What
//! the attachments' own drop does is not this — that is the browser's
//! `DataTransfer`, for files coming from outside the page.
//!
//! **Where it lands is a place along a tab bar**, its own or any other group's,
//! marked with a line while the hand is over it so that what will happen is
//! drawn before the hand lets go. Dropping reorders the tab in its own bar, or
//! takes it out of one group and into another at that place and makes that
//! group the active one; a drag that comes back to where it started changes
//! nothing, and one let go of away from every bar and every group changes
//! nothing either. A group whose last tab has just been dragged out of it
//! disappears, which is the rule its last tab being closed already follows.
//!
//! **Or one of five zones of a group's content**, which is the other half of
//! where a tab can land. The centre moves it into that group, the way a place
//! along that group's bar does; each of the four edges splits the group there,
//! with the dragged tab alone in the new half and the rest of the group's tabs
//! staying where they were. The side the hand pointed at is the side the new
//! group takes, and the split is made where the group stands in the tree, so an
//! edge drop on a group that is already half of one nests a level rather than
//! adding a sibling. The zone under the hand is drawn while it is over one —
//! the centre as the whole of the content, an edge as the band the new group
//! would take — and one zone anywhere at a time, a pointer being in one place.
//!
//! **And a file row of the tree is picked up the same way**, onto those same
//! two targets: the same grace before a press becomes a drag, the same hold to
//! lift one under a finger, the same line along a bar and the same five zones
//! of a group's content. What a drop does is open the file — in the group whose
//! bar or centre it landed on, or in the new group an edge makes — so dragging
//! one in is the press that opens a file pointed at a group rather than at the
//! active one. A press that let go about where it landed is still that press,
//! and a folder row is not a drag source, there being nothing to open. Nothing
//! here writes anything: the tree is exactly as it was whatever the drag came
//! to, dragging being a way to open a file rather than a move on disk.
//!
//! **And a view is drawn once and placed into the group holding it**, which is
//! what lets a tab move without being made again. A terminal's tab carries a
//! live socket and this window's own memory of what has scrolled past it, and a
//! file's carries a caret and an undo stack: a move that drew the tab afresh in
//! its new group would close the socket, throw the scrollback away and reattach
//! to a repaint. So a tab *is* the view — one group holds it, one box is drawn
//! for it, and a drag moves both — and the pane puts that box in whichever
//! group's content it belongs to, the way it places the groups themselves
//! rather than nesting them.
//!
//! **The active group is the one last pressed into**, wherever in it the press
//! landed, and it is where an opening goes: a file pressed in the tree, a
//! terminal asked for, the tab standing on a shell that would not start. A
//! split's own new group is active, that being where the work was going.
//!
//! **And five keystrokes act on it, from wherever in the pane the hands are.**
//! Ctrl+PageDown and Ctrl+PageUp turn the active group to the next tab and to
//! the one before it, wrapping at each end; Ctrl+\ splits it beside itself, the
//! split its bar's icon and its tab menu make; Ctrl+` opens a shell in it, the
//! one **New terminal** opens; and Ctrl+P drops the quick-open palette over the
//! pane, which opens what it is given into that same group. They hang on the document where Ctrl+S already
//! hangs, for as long as Code is mounted, which is the whole of their reach —
//! so a press arrives whichever half of the pane it was made in, and what it
//! acts on is the active group rather than whatever has the focus. Ctrl+W and
//! Ctrl+Tab are the browser's own and are deliberately not taken: a page that
//! swallowed either would be a pane fighting the window around it. And none of
//! them is a key a terminal grid already had — Ctrl+C interrupts, Ctrl+D ends,
//! Ctrl+L clears, and all of those go up the socket untouched.
//!
//! **A file pressed in the tree opens as a tab of the active group.** What the
//! server answers a read with says which of four kinds of thing it read — text,
//! an image, a binary it will not send, or a file over the size cap — and the
//! tab draws each as what it is: the text in an editor, the picture in its tab,
//! and either of the last two as the line saying why there is nothing to draw.
//! A file in a read-only root opens read-only and takes no typing, which is the
//! root's own flag rather than the file's mode.
//!
//! **And one is opened by name as well, out of the quick-open palette** — see
//! [`./Quick`], which Ctrl+P drops over the whole pane. It matches file names
//! across every root the Conversation has, on a list read when it opens, and
//! what Enter takes opens into the active group exactly as a press in the tree
//! does: the tree is how a file is found by where it sits, and this is how one
//! is found by what it is called (ADR 0019, *The tree*). Escape leaves nothing
//! behind at all.
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
//! text the *next* save is of.
//!
//! **And the tabs follow the disk**, which is where that bar is usually drawn
//! from: the watcher says this Conversation's Worktrees moved, and every open
//! file is read again (ADR 0019, *Following the disk*). The version is what
//! tells a real change from a touch — where it matches, nothing happens at all,
//! which is nearly every file on nearly every Nudge. Where it differs, a clean
//! buffer takes the new text silently with its caret kept as near as it can be,
//! and a dirty one keeps the human's text and raises the bar the moment the
//! disk moves rather than at the next Ctrl+S. A file that has gone keeps its
//! tab with its text in it, the way one deleted out of the tree does. See
//! [`again`], and the subscription beside it — the pane's own rather than a row
//! of the table in `nudge.ts`, the readings being held above this pane with the
//! tabs.
//!
//! **The pane's own save is not the disk moving under it.** A write that landed
//! answers with the version it made, so the Nudge that write raises finds the
//! file reading at the version the tab is already standing on — and a file with
//! a save still in flight is not read at all, so the answer cannot arrive before
//! the write's does.
//!
//! **And the same file opened twice is one buffer.** Pressing a file already
//! open in the active group turns to its tab rather than opening a second
//! beside it; pressed while it is open in another group, it opens here as well
//! — two tabs, one buffer, and two views of the one text. Typing in either
//! shows in the other, the dot is on both tabs and one save clears both.
//!
//! **And every open tab follows its file when the tree renames one.** The tab
//! is retitled, its reading is re-keyed onto the new path and its buffer is
//! re-made there — a Monaco model is registered at the file's own address and
//! the package cannot rename one, so the model is disposed and made again with
//! the same text in it. What carries over is the text and whether it is dirty;
//! what goes is the undo stack, which is Monaco's limit rather than a choice.
//! The next Ctrl+S writes to the new path. A folder renamed carries everything
//! under it — every open tab whose path was inside it, and every folder of the
//! tree expanded beneath it — and the same file open in two groups is one
//! buffer under both, so both tabs follow together. See [`renamed`], which is
//! where every place the path was written down is moved.
//!
//! **And a deleted file's tab stays**, read-only, saying the file is gone, with
//! whatever was in it still there to be read and copied out. A tab that vanished
//! under somebody with unsaved text would take the text with it, so the row goes
//! from the tree and the tab does not: Ctrl+S in it writes nothing — there is
//! nothing to write over — and the × closes it the way it closes any other tab,
//! asking first, because what it throws away is text the disk has not got. A
//! folder deleted carries every tab inside it, the way a folder renamed does.
//! See [`deleted`].
//!
//! **And it survives a reload**, because the text of a file the disk has not got
//! is exactly what this device writes down: a page that comes back reads the
//! file, finds it missing, and puts the remembered text into a tab drawn as gone
//! rather than as a refusal to read — which is what makes *copy it out later* a
//! promise rather than a hope.
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
//! is kept).
//!
//! **And through a reload as well**, which is the device's storage rather than
//! the page's: the layout, each group's tabs and the one it is showing, and the
//! text of every buffer the disk has not got, written down per Conversation in
//! the browser (see [`./remembering`]). What is *not* written down is what the
//! disk said — so every restored file is read here on the way in and the human's
//! text goes over the top of it, which is what makes a file that moved while the
//! page was away dirty against what is really there. And a page left with text
//! nobody has saved still warns on the way out, the browser's own way, because
//! a browser that refuses storage is a browser that loses it.
//!
//! The terminals are the exception, and the register is why: a shell is the
//! server's, so the list is read on every opening and the tabs are settled
//! against it. That settling walks the tree rather than a list: one whose shell
//! has ended since is dropped from whichever group holds it — and the group
//! with it, where it was that group's last tab — and one the register has that
//! no group does joins the active group, which is how a shell opened on another
//! device arrives.
//!
//! **And the pane's own ⋯ in the header**, which is where what is about the
//! pane rather than about anything open in it goes: the three settings ADR 0019
//! exposes — word wrap, the font size and the minimap — each a row of the one
//! menu the app has. Kept per device beside the Diff's own wrap setting (see
//! `device.ts`) and never sent to the server, for the reason that one is: a
//! phone and a laptop are entitled to draw the same file differently, and
//! neither has any business deciding for the other. A browser that refuses
//! storage costs the setting and nothing else, the editors drawing VS Code's
//! defaults, which is what an untouched browser already gets.
//!
//! **And the settings are the pane's rather than an editor's**, which is what
//! makes a press reach every editor open in every group at once, live, with no
//! tab reopened — the way the light and dark themes already do. One signal
//! here, read by each editor as it is drawn: the menu writes it and the tabs
//! below it read it, and they are two halves of the one pane.
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
  Index,
  Match,
  Show,
  Switch,
  batch,
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import { Icon } from "../Icon";
import { IconButton } from "../IconButton";
import { ContextMenu, Menu, Nested } from "../Menu";
import { Modal } from "../Modal";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
import {
  closeTerminal,
  filesSocket,
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
  FolderListing,
  TerminalOpened,
} from "../api/types";
// The three settings the pane's own menu carries, where this device keeps them.
// Renamed on the way in: the pane holds them in a signal of its own, and the
// two halves of that — what was read at rest, and the writing down of a change
// — read better here than the storage's own names would beside it.
import {
  SIZES,
  drawn as atRest,
  setDrawn as remember,
  type Drawn,
} from "../device";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
// The seam beside the nudge table: what the tabs and the tree follow the disk
// by, the readings under them being held above this pane rather than in a
// query — see `whenFilesMove`, which is where the reasoning is.
import { whenFilesMove } from "../nudge";
import { keyOf, useDevice } from "../reaching";
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
  borders,
  clamped,
  found,
  groups as groupsOf,
  moved,
  neighbour,
  nudged,
  placed,
  split,
  without,
  type Border,
  type Group,
  type Layer,
  type Way,
} from "./layout";
import { PaneHead } from "./PaneHead";
import { Quick } from "./Quick";
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

/// And what the bar over a file that moved under unsaved text says.
///
/// The collision itself, in the words of what happened rather than of what went
/// wrong: the agent writes the same Worktree, and a file it rewrote while
/// somebody had it open is the ordinary way of things rather than a fault. What
/// the two presses under it do is named in them (ADR 0019, *Versioned reads,
/// and a stale write is refused*).
///
/// One sentence for both ways it is raised — the watcher saying the file moved,
/// and a save refused for the same reason — because they are the one fact:
/// the disk has something else in it, and what is in the editor is not on it.
export const MOVED =
  "This file changed on disk while you were editing it, so nothing was saved.";

/// How long the pane waits before dialling its attachment again when the socket
/// closes under it, and how far that wait doubles out to.
///
/// **A socket closing is not the pane going.** That it dies with the tab is the
/// whole reason it is what says a pane is drawn — but a server restarted under
/// an open pane closes it too, and so does a connection that dropped or a
/// tunnel that blinked, and a pane that read any of those as a detach would go
/// on drawing a tree that never followed the disk again. Nothing would say so
/// either: the socket carries nothing, so there is no message missing and no
/// sentence to draw.
///
/// So it is dialled again for as long as the pane is drawn. Quickly at first,
/// because a server taken down for an update is seconds; backing off to a few
/// seconds after that, because one that is not coming back is not worth dialling
/// in a loop. A connection that lands puts it back to the first wait, so a blip
/// an hour later is answered as quickly as this one was.
const ATTACH_AGAIN = 250;
const ATTACH_AT_MOST = 10_000;

/// And what a tab whose file has gone says, over the text it still has.
///
/// Drawn as a bar above the editor rather than in place of it, which is the
/// whole of the point: the file is gone and the text is not, and what the human
/// has to be told is that the second is now the only copy (ADR 0019, *The
/// tree*). Said where the *Reload* / *Keep mine* bar is said, being the other
/// thing that is true of the file rather than of the editor.
///
/// Not one of [`FILE_REFUSAL`]'s: `Missing` there is a file that was never
/// opened, and the tab has nothing in it to keep. This is the same answer read
/// against a buffer that is still holding text — see [`Opened`], where the two
/// are told apart.
export const GONE =
  "This file is gone from the disk. What is here is the text as it was, kept so it can be copied out; nothing can be saved to it.";

/// What a tab is known by, which is the one string that can stand for either.
///
/// Which tab is showing, and which the human turned to, are both this rather
/// than the tab itself: a path and a number would be two signals holding one
/// answer between them.
export function keyed(tab: Tab): string {
  return "terminal" in tab ? `terminal ${tab.terminal}` : `file ${tab.file}`;
}

/// And the same thing open again, as a second view of it.
///
/// A tab object is one **view**: the thing that stands in exactly one group, has
/// exactly one box drawn for it, and travels through the tree when somebody
/// drags it. What it is a view *of* is [`keyed`] — a path, or a number — which
/// is what two views of one file have in common and what makes them one buffer.
///
/// So a split, which puts what a group is showing in both halves, makes a
/// second tab rather than putting the one tab in two groups: two views of the
/// one file or the one shell, which is what ADR 0019 asks for, and two things
/// the pane can tell apart when one of them is dragged somewhere.
export function copied(tab: Tab): Tab {
  return "file" in tab ? { file: tab.file } : { terminal: tab.terminal };
}

/// And what a file's tab is called: its name.
///
/// Rather than its path — a tab is a few rems wide, a path in a checkout is a
/// sentence, and where a file sits is what the tree beside it says. A path of
/// nothing but separators has no name, and reads as itself.
export function named(path: string): string {
  return path.split(/[/\\]/).filter(Boolean).pop() ?? path;
}

/// A record of things named by path, with the paths a rename reached moved onto
/// where they now are.
///
/// Written once because the pane keeps several of them — what each open file was
/// read as, what its last save came to, which folders of the tree are open — and
/// a rename moves the key in every one of them the same way. What is under the
/// key is left as it was unless `inside` is given, which is for the two that name
/// their own path a second time within themselves.
function rekeyed<T>(
  was: Record<string, T>,
  moved: (path: string) => string | null,
  inside?: (held: T, at: string) => T,
): Record<string, T> {
  return Object.fromEntries(
    Object.entries(was).map(([path, held]) => {
      const at = moved(path);

      return at === null
        ? [path, held]
        : [at, inside === undefined ? held : inside(held, at)];
    }),
  );
}

/// One reading with the path it names moved onto where the file now is.
///
/// A reading that went on naming where the file *was* would be the one thing
/// left in the pane still saying so — the text, the version and the bytes are
/// all about the file wherever it has got to. The refusals name nothing, being
/// words.
function atPath(reading: FileReading, at: string): FileReading {
  if (typeof reading === "string") {
    return reading;
  }

  if ("Text" in reading) {
    return { Text: { ...reading.Text, path: at } };
  }

  return "Image" in reading ? { Image: { ...reading.Image, path: at } } : reading;
}

/// The version a reading carries, where it is text and so carries one.
///
/// A hash of the bytes that were read, which is what a write names itself as
/// being over — and what tells a file that really moved from one that was
/// written with what it already held (ADR 0019, *Following the disk*).
function versioned(reading: FileReading): string | undefined {
  return typeof reading !== "string" && "Text" in reading
    ? reading.Text.version
    : undefined;
}

/// And whether a file read again came back saying what the tab is already
/// standing on.
///
/// Asked of every open file on every `files` Nudge, and true of nearly all of
/// them: a Nudge says the Worktrees moved and nothing about where, so a build
/// filling a folder is a read apiece that comes back to this and leaves every
/// tab exactly as it is.
///
/// **The version is the whole of it wherever there is one.** It is a hash of
/// the bytes, so a file rewritten with what it already held reads the same and a
/// timestamp moved is not a change at all — which is the difference between a
/// tab that takes new text and one that does nothing, and between a bar raised
/// over the human's typing and no bar. Where neither side has one — a picture,
/// a file that has gone, a refusal — it is what the two *say*, the way the tree
/// compares its listings: both are the one endpoint's own JSON, written in the
/// one order.
function same(was: FileReading, now: FileReading): boolean {
  const before = versioned(was);
  const after = versioned(now);

  return before === undefined && after === undefined
    ? JSON.stringify(was) === JSON.stringify(now)
    : before === after;
}

/// And one folder listing with every path in it moved: the folder it lists, and
/// each row it holds.
///
/// The rows are what a press in the tree opens, so a listing still naming the
/// old paths after its folder moved would be a column of rows that are not
/// there.
function listed(
  listing: FolderListing,
  moved: (path: string) => string | null,
): FolderListing {
  if (typeof listing === "string" || !("Listed" in listing)) {
    return listing;
  }

  const { path, entries } = listing.Listed;

  return {
    Listed: {
      path: moved(path) ?? path,
      entries: entries.map((entry) => ({
        ...entry,
        path: moved(entry.path) ?? entry.path,
      })),
    },
  };
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
  const device = useDevice();

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
  // Under the device with the Conversation, for the reason every key here
  // carries one: ids collide by construction, and a register keyed by a bare id
  // would offer a member's shells as this device's.
  const holding = () => keyOf(device(), "terminals", props.conversation.id);

  const terminals = useReading(() => ({
    queryKey: holding(),
    queryFn: () => listTerminals(device(), props.conversation.id),
    freshness: "static",
    gcTime: 0,
  }));

  onCleanup(() => held.removeQueries({ queryKey: holding(), exact: true }));

  // And the word to the server that this pane is drawn, which is the whole of
  // how the disk is followed (ADR 0019, *Following the disk*): the server
  // watches this Conversation's worktrees while a Code pane is attached and not
  // otherwise, and what says one is, is a socket held open for as long as it
  // stands.
  //
  // Nothing travels either way and nothing is drawn about it. What moving on
  // disk comes to is a `files` Nudge down the stream every other change comes
  // down, and what that comes to is `nudge.ts`'s: the roots off its table, and
  // the folders the tree has expanded off the subscription the tree takes out
  // beside it. So a watcher that never started costs this pane a tree that does
  // not follow rather than anything to say.
  //
  // Held open rather than asked for and renewed, the way a terminal tab holds
  // its attach: it dies with the tab whatever becomes of the browser, so a
  // laptop shut mid-edit stops the watcher without anybody having to notice.
  // And let go of on the way out, which is a details pane swapped for an Event
  // as much as it is one closed — the server waits a moment for the pane to
  // come back before it stops anything, so a swap costs no watcher.
  //
  // And dialled again where it closes under a pane that is still drawn, which is
  // a restarted server or a connection that dropped rather than a detach — see
  // [`ATTACH_AGAIN`], which is where the waits and the reason for them are.
  createEffect(() => {
    const at = filesSocket(device(), props.conversation.id);

    let attached: WebSocket | undefined;
    let waiting: ReturnType<typeof setTimeout> | undefined;
    let again = ATTACH_AGAIN;
    // Whether the pane has gone, which is the one closing that is not dialled
    // again: a `close()` of this pane's own making raises the same event a
    // server going away does, and there is nothing left here to attach for.
    let over = false;

    const dial = (): void => {
      const socket = new WebSocket(at);
      attached = socket;

      // A connection that landed, so the next one to drop is answered as
      // quickly as this one was.
      socket.addEventListener("open", () => {
        again = ATTACH_AGAIN;
      });

      socket.addEventListener("close", () => {
        if (over || socket !== attached) {
          return;
        }

        waiting = setTimeout(dial, again);
        again = Math.min(again * 2, ATTACH_AT_MOST);
      });
    };

    dial();

    onCleanup(() => {
      over = true;
      clearTimeout(waiting);
      attached?.close();
    });
  });

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
    recall,
    carry,
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

  /// How every editor in the pane is drawn: word wrap, the font size and the
  /// minimap, which are the three settings ADR 0019 exposes.
  ///
  /// **Here rather than in an editor**, which is the whole of what makes a
  /// press on the menu reach every editor open in every group at once: one
  /// answer, read by each of them, so none of them has an opinion of its own to
  /// fall out of step with the others. The menu in the header writes it and the
  /// tabs below read it, and they are two halves of the one pane.
  ///
  /// **And read off the device as the pane is built**, the way the maximise
  /// toggle beside it is: a reload comes back to what was set, and a browser
  /// with no storage to read comes back to VS Code's defaults rather than to a
  /// failure. Per device and never sent to the server — a phone and a laptop
  /// are entitled to draw the same file differently.
  const [drawing, setDrawing] = createSignal(atRest());

  /// Move one of the three, and remember it for the next pane this device
  /// opens.
  ///
  /// One setting at a time, which is what a row of the menu is: the other two
  /// are carried over rather than said again, so a press cannot quietly put one
  /// of them back.
  const draw = (change: Partial<Drawn>): void => {
    const settled = { ...drawing(), ...change };

    setDrawing(settled);
    remember(settled);
  };

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

  /// And every view there is, which is every tab of every group: one box is
  /// drawn per entry here, wherever the group holding it stands.
  ///
  /// A tab *is* a view — see [`copied`] — so this is a flat list of the things
  /// the pane draws content for, and a tab dragged from one group to another is
  /// the same entry with a different group around it rather than one leaving
  /// this list and another joining it.
  const viewed = createMemo(() => groups().flatMap((one) => one.tabs()));

  /// Where each group's views are put: the box inside it that its content goes
  /// in, by the group's id.
  ///
  /// Written by the groups as they are drawn and read by the views as they are
  /// placed, which is the one thing the two halves say to each other.
  const [slots, setSlots] = createSignal<Record<number, HTMLElement>>({});

  /// The layer every group is drawn in, which is the ground their percentages
  /// are measured against — and, once it has been measured, what turns the
  /// floors under those percentages into shares.
  let layer!: HTMLDivElement;

  /// How large it stands, in rem. Nought until the page has laid out, which the
  /// arithmetic reads as no floors yet rather than as floors of nothing (see
  /// [`./layout`]).
  ///
  /// In rem rather than pixels, and converted here where the browser can be
  /// asked what a rem is: a human who has told their browser to draw text
  /// larger has said a group should hold what it held.
  const [across, setAcross] = createSignal(0);
  const [down, setDown] = createSignal(0);

  const measure = (box = layer.getBoundingClientRect()) => {
    const root =
      parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

    if (box.width > 0) {
      setAcross(box.width / root);
    }

    if (box.height > 0) {
      setDown(box.height / root);
    }
  };

  /// Measured when the pane is first drawn, and again whenever the layer
  /// changes shape under the groups — a window dragged narrower is exactly
  /// when a group stops being wide enough for a bar. A browser with no
  /// `ResizeObserver` to ask keeps the size it opened at, and measures again at
  /// the start of every drag. The frame's own dividers say all of this the same
  /// way, in `Panes.tsx`.
  onMount(() => {
    measure();

    if (typeof ResizeObserver !== "function") {
      return;
    }

    const watching = new ResizeObserver(() => measure());

    watching.observe(layer);
    onCleanup(() => watching.disconnect());
  });

  const sized = (): Layer => ({ width: across(), height: down() });

  /// The tree as it is actually drawn: every share met against the floors under
  /// the groups either side of it, given how large the layer stands now.
  ///
  /// Which is why a window changing shape moves nothing. What the layout holds
  /// is where the human left each border, and this is that held against the
  /// room there is — so a layer that grows again hands the share straight back.
  const drawn = createMemo(() => clamped(layout(), sized()));
  const placing = createMemo(() => placed(drawn()));

  /// And every border between them, which is where the dividers go: one per
  /// split, carrying the share it decides and how far it may be taken.
  const bordering = createMemo(() => borders(drawn(), sized()));

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

  /// And where a border stands: the line itself, across the room its split
  /// divides, at the share the half before it is worth.
  ///
  /// A line rather than a box — it is the one thing in the layer with no room
  /// of its own, the groups either side of it having every percent between
  /// them. What gives it something to take hold of is the half-rem the
  /// stylesheet centres on it, which is the frame's divider again.
  const along = (border: Border): JSX.CSSProperties => {
    const { x, y, width, height } = border.within;

    return border.way === "beside"
      ? {
          left: `${x + (width * border.share) / 100}%`,
          top: `${y}%`,
          height: `${height}%`,
        }
      : {
          left: `${x}%`,
          top: `${y + (height * border.share) / 100}%`,
          width: `${width}%`,
        };
  };

  /// Dragging one, which is the frame's own divider again (`Panes.tsx`): the
  /// listeners go on the window rather than on the handle, because a pointer
  /// that has outrun the line — which every drag's does — is still dragging it.
  ///
  /// What the pointer is measured against is the layer, and what comes of that
  /// is a share of the one split this border divides: the half before it takes
  /// what the half after it gives, and nothing else in the tree moves.
  const drag = (border: Border, event: PointerEvent) => {
    // Which stops the drag selecting the text of both groups on the way past.
    event.preventDefault();

    const box = layer.getBoundingClientRect();
    const across = border.way === "beside";
    const room = across ? border.within.width : border.within.height;

    if (box.width === 0 || box.height === 0 || room === 0) {
      return;
    }

    // Free, the box being in hand: it keeps a browser with no observer to ask
    // from meeting the floors against a layer the window has resized out from
    // under.
    measure(box);

    const moving = (at: PointerEvent) => {
      const point = across
        ? ((at.clientX - box.left) / box.width) * 100
        : ((at.clientY - box.top) / box.height) * 100;
      const share =
        ((point - (across ? border.within.x : border.within.y)) / room) * 100;

      setLayout((was) => moved(was, border, share));
    };

    const dropped = () => {
      window.removeEventListener("pointermove", moving);
      window.removeEventListener("pointerup", dropped);
      window.removeEventListener("pointercancel", dropped);
    };

    window.addEventListener("pointermove", moving);
    window.addEventListener("pointerup", dropped);
    window.addEventListener("pointercancel", dropped);
  };

  /// And moving one with the keyboard, along the axis its own split divides:
  /// a border down the middle answers left and right, and one across answers
  /// up and down. The same travel a drag gives it, for the pointer nobody
  /// dragging with a keyboard has.
  const nudge = (border: Border, event: KeyboardEvent) => {
    const keys =
      border.way === "beside"
        ? { ArrowLeft: -1, ArrowRight: 1 }
        : { ArrowUp: -1, ArrowDown: 1 };
    const by = (keys as Record<string, number | undefined>)[event.key];

    if (by === undefined) {
      return;
    }

    event.preventDefault();
    setLayout((was) => nudged(was, border, by));
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

    // A second view of the same thing rather than the same tab in two groups —
    // see [`copied`]: a tab is what one group holds and what a drag carries,
    // and one object in two bars would be one box the pane could not place.
    made.setTabs([copied(tab)]);
    made.setChosen(keyed(tab));

    setLayout((was) => split(was, group.id, way, made));
    setActive(made.id);
  };

  /// Which tab the hand is carrying, and where it would land if it let go now.
  ///
  /// The tab is what the bar draws as lifted, and the landing is what the pane
  /// marks: a line along a bar, or the band of a group's content. Both null
  /// every moment nobody is dragging one, which is nearly all of them.
  const [carried, setCarried] = createSignal<Tab | null>(null);
  const [mark, setMark] = createSignal<Landing | null>(null);

  /// And which file the tree has had picked up out of it, where the hand is
  /// carrying one of those instead.
  ///
  /// A path rather than a tab, because nothing has been opened yet: what is in
  /// the hand is the row, and the tree draws it lifted the way a bar draws the
  /// tab in one. One or the other of these two is set while a drag is on, and
  /// neither of them the rest of the time.
  const [taken, setTaken] = createSignal<string | null>(null);

  /// The press in flight: what it began on — a tab of a group, or a file row of
  /// the tree — where on the screen it began, whether it is a finger, and
  /// whether what it holds has lifted under it yet. Null every moment nothing
  /// is pressed.
  ///
  /// Not a signal, because nothing is drawn from it — what the lifted tab and
  /// the insertion point are drawn from are the two above, and the rest of this
  /// is bookkeeping between one pointer event and the next. The sidebar's cards
  /// are picked up exactly this way, in `Conversations.tsx`.
  let press: {
    what: Carrying;
    pointer: number;
    x: number;
    y: number;
    touch: boolean;
    lifted: boolean;
    waiting?: ReturnType<typeof setTimeout>;
  } | null = null;

  /// What takes a drag's listeners back off the window, or null while nothing
  /// is pressed. They are made per press — each closes over the press it
  /// belongs to — so what removes them is made alongside them.
  let stop: (() => void) | null = null;

  /// What the press that has just ended was carrying, where it was a drag at
  /// all. The click arrives after the pointer is up, and a thing carried into
  /// place should not be answered a second time: a tab turned to as well, or a
  /// file opened as well in the group its row was dragged out of the tree past.
  ///
  /// The thing itself rather than a flag, because what says a click belongs to
  /// this drag is that it is a click on the very tab, or the very row, that was
  /// carried: a flag would swallow whatever was pressed next instead. Spent by
  /// that click, and cleared by the next press whatever it turns out to be, so
  /// a drag that ended with no click behind it leaves nothing standing — a tab
  /// then deaf to Enter, or a file row that would not open.
  let carrying: Carrying | null = null;

  /// It lifts: the tab is being moved, or the file carried in, from here until
  /// the hand lets go.
  const lift = (at: NonNullable<typeof press>): void => {
    at.lifted = true;

    if ("tab" in at.what) {
      setCarried(at.what.tab);
    } else {
      setTaken(at.what.path);
    }

    // The bar must not scroll out from under a tab being moved along it. A
    // `touch-action` on the tab would have said so before the finger landed and
    // taken the swipe that scrolls the bar with it, so the scroll is refused
    // here instead: from the lift until the hand lets go, and never while a
    // finger is merely passing through.
    if (at.touch) {
      document.addEventListener("touchmove", refuseScroll, { passive: false });
    }
  };

  /// A press begins on a tab. Which of the three things it is — a press that
  /// turns to the tab, a scroll of the bar, or a drag — is settled by what the
  /// hand does next, which is how a card in the sidebar is picked up.
  const grab = (event: PointerEvent, group: Group, tab: Tab): void => {
    // Which hand this is, before anything is decided about the press: the menu
    // a right-click drops is the mouse's alone, and the `contextmenu` behind it
    // is the one thing that cannot say which hand made it.
    fromTouch = event.pointerType !== "mouse";

    begin(event, { group, tab });
  };

  /// And a press begins on a file row of the tree, which is the same gesture
  /// carrying a path rather than a view of one.
  ///
  /// Which of three things *this* one is — a press that opens the file, a
  /// scroll of the tree, or a drag — is settled the same way, by what the hand
  /// does next. Nothing sets [`fromTouch`] here: what that decides is whether a
  /// `contextmenu` is a right-click, and the tree has no menu to drop.
  ///
  /// A folder row does not call this at all. There is nothing to open, so there
  /// is nothing for a drop to do, and a press on one is the expand it has
  /// always been.
  const pick = (event: PointerEvent, path: string): void => {
    begin(event, { path });
  };

  /// The gesture itself, whichever of the two began it — and from here down
  /// everything is written of what is in the hand rather than of where it came
  /// from, because a tab and a file row are dropped onto the same places.
  const begin = (event: PointerEvent, what: Carrying): void => {
    // The primary button, a finger or a pen. A right-click is not a drag.
    if (event.button !== 0) {
      return;
    }

    // A press whose ending never reached us is over the moment another begins.
    // Nothing should get this far with one still in flight — every way a drag
    // can end is listened for below — and one left standing would be a bar held
    // by a hand that is no longer on it.
    put();

    // The tab takes the pointer for as long as the browser will leave it there,
    // so nothing it is carried over lights up under a hand that is already
    // holding something. For as long as it will leave it and no longer: the bar
    // moving this very tab is what the drag is for, and an element that moves
    // in the DOM has the pointer taken back off it. So the capture is a
    // courtesy — what the drag runs on is the window, which hears the pointer
    // whoever is holding it.
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);

    const began: NonNullable<typeof press> = {
      what,
      pointer: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      touch: event.pointerType !== "mouse",
      lifted: false,
    };
    press = began;

    // A drag that ended with no click behind it — a release the browser took
    // over, a tab that went with the group it emptied — has nothing left to
    // swallow, and this is where that is settled: a press is the one moment
    // that says the gesture before it is over.
    carrying = null;

    // The rest of the gesture is watched at the window rather than at the tab,
    // which is what the dividers beside it do: a pointer that has outrun the
    // tab is still dragging it, and a release out beyond the pane — or beyond
    // the window — is still the release. A cancel is an ending too, being what
    // the browser says when it has taken the gesture over. Both of them put the
    // tab down, so there is no way for a drag to end that leaves one held.
    const moving = (at: PointerEvent) => {
      if (at.pointerId === began.pointer) {
        haul(at);
      }
    };
    const ended = (at: PointerEvent) => {
      if (at.pointerId === began.pointer) {
        put();
      }
    };

    stop = () => {
      window.removeEventListener("pointermove", moving);
      window.removeEventListener("pointerup", ended);
      window.removeEventListener("pointercancel", ended);
    };

    window.addEventListener("pointermove", moving);
    window.addEventListener("pointerup", ended);
    window.addEventListener("pointercancel", ended);

    // A finger lifts what it is holding by holding still. No distance tells a
    // drag from a scroll of the bar — or of the tree — on a phone, both of them
    // being the finger moving, so what tells the two apart is the time before
    // it does.
    if (began.touch) {
      began.waiting = setTimeout(() => {
        if (press === began) {
          lift(began);
        }
      }, LIFT);
    }
  };

  /// The hand moved: past the grace it is a drag, and a drag marks where the
  /// tab would land.
  const haul = (event: PointerEvent): void => {
    const at = press;

    if (at === null) {
      return;
    }

    if (!at.lifted) {
      // Inside the grace the hand has said nothing yet: this is the wobble
      // between pressing a tab and letting go of it, and a tab that started
      // moving here is a tab that could not be pressed at all.
      if (Math.hypot(event.clientX - at.x, event.clientY - at.y) <= GRACE) {
        return;
      }

      // A finger that travels before what it holds has lifted is scrolling the
      // bar, or the tree, so the press is over and the browser has it. Nothing
      // has lifted, so ending it here moves nothing.
      if (at.touch) {
        put();
        return;
      }

      lift(at);
    }

    setMark(landing(event.clientX, event.clientY));
  };

  /// The drag is over: what was in the hand goes where the mark stood — the tab
  /// moved there, or the file opened there.
  ///
  /// Every ending comes through here — the release, a cancel, a press that
  /// turned out to be a scroll, and the next press finding this one standing —
  /// so there is one place the listeners come off and one place the hand is
  /// emptied. A release away from every bar and every group is an ending like
  /// any other, and does nothing: what it was let go over is what says what a
  /// release does, and out there is nothing.
  const put = (): void => {
    const at = press;
    const where = mark();

    stop?.();
    stop = null;
    press = null;
    setCarried(null);
    setTaken(null);
    setMark(null);

    if (at === null) {
      return;
    }

    clearTimeout(at.waiting);

    if (!at.lifted) {
      return;
    }

    document.removeEventListener("touchmove", refuseScroll);
    carrying = at.what;

    if (where === null) {
      return;
    }

    if (!("tab" in at.what)) {
      bring(at.what.path, where);
      return;
    }

    if ("at" in where) {
      move(at.what.group, at.what.tab, where.group, where.at);
    } else {
      onto(at.what.group, at.what.tab, where.group, where.zone);
    }
  };

  /// A tab put down at a place along a bar: its own, which is a reorder, or
  /// another group's, which takes it out of one and into the other.
  ///
  /// The group it lands in becomes the active one — it is where the work has
  /// just gone — and a group whose last tab has left disappears, which is the
  /// rule every other way of emptying one already follows.
  ///
  /// **And a tab put down where the same thing is already open is the two views
  /// becoming one**: the tab that was carried goes, and the group turns to the
  /// one it already had. Two tabs of one file in a bar would be two of
  /// everything that bar says about it — two dots, two marks of which is
  /// showing — over the one buffer, and there is nothing a second view of a
  /// file in the same group could show that the first is not showing already.
  ///
  /// In one go, because the tab is out of every group between the two writes:
  /// the pane draws a box per view and reads the views off the groups, so a
  /// moment with the tab in neither is that box taken down and made again —
  /// which is the socket and the scrollback this whole arrangement is here to
  /// keep.
  const move = (from: Group, tab: Tab, to: Group, at: number): void => {
    const stood = from.tabs().indexOf(tab);

    if (stood < 0) {
      return;
    }

    if (from === to) {
      // The place was read off the bar as it stands, with the tab still in it,
      // so a tab let go of beyond its own place has one fewer tab in front of
      // it once it is taken out. Which is also what makes a drag that came back
      // to where it started come to nothing at all.
      const landing = at > stood ? at - 1 : at;

      if (landing === stood) {
        return;
      }

      from.setTabs((was) => {
        const put = [...was];

        put.splice(stood, 1);
        put.splice(landing, 0, tab);

        return put;
      });

      return;
    }

    batch(() => {
      const already = to.tabs().some((one) => keyed(one) === keyed(tab));

      from.setTabs((was) => was.filter((one) => one !== tab));

      if (!already) {
        to.setTabs((was) => [...was.slice(0, at), tab, ...was.slice(at)]);
      }

      to.setChosen(keyed(tab));

      if (from.tabs().length === 0) {
        shut(from);
      }

      setActive(to.id);
    });
  };

  /// A tab put down on what a group is *showing*, which is the third of the
  /// three ways to make a split (ADR 0019, *Tabs and groups*).
  ///
  /// The centre moves it into that group, the way a place along its bar does
  /// and with the same rules under it: the group it lands in becomes active, a
  /// group whose last tab has just left disappears, and the tab it is carrying
  /// keeps its socket or its caret, being the one view it always was.
  ///
  /// Each edge splits the group there instead, with the dragged tab alone in
  /// the new half and every other tab of the group staying where it was. The
  /// side the hand pointed at is the side the new group takes, which is what
  /// tells left from right and above from below — and the split is made where
  /// the group stands in the tree, so an edge drop on a group that is already
  /// half of one nests a level rather than adding a sibling.
  const onto = (from: Group, tab: Tab, to: Group, zone: Zone): void => {
    if (zone === "centre") {
      // Already in the group the hand let it go over. A centre drop asks for
      // the group rather than for a place along its bar, so there is nothing to
      // move and what is left to answer is which tab the group shows — which is
      // what a lone tab dropped on its own group's content comes to.
      if (from === to) {
        batch(() => {
          to.setChosen(keyed(tab));
          setActive(to.id);
        });

        return;
      }

      // At the end of the bar, there being no place along one in this gesture:
      // what the hand pointed at was the group.
      move(from, tab, to, to.tabs().length);

      return;
    }

    const way: Way = zone === "left" || zone === "right" ? "beside" : "below";

    // A group's only tab dropped on an edge of that same group asks for the
    // group it already is: the new half would hold everything the old one held,
    // and the old one would go the moment it was made.
    if (from === to && from.tabs().length < 2) {
      return;
    }

    const made = fresh();

    // In one go, for the reason a move between two bars is: the pane draws a
    // box per view and reads the views off the groups, so a moment with the tab
    // in neither group is that box taken down and made again — which is the
    // socket and the scrollback this arrangement is here to keep.
    batch(() => {
      from.setTabs((was) => was.filter((one) => one !== tab));

      made.setTabs([tab]);
      made.setChosen(keyed(tab));

      setLayout((was) =>
        split(was, to.id, way, made, zone === "left" || zone === "above"),
      );

      if (from.tabs().length === 0) {
        shut(from);
      }

      setActive(made.id);
    });
  };

  /// A file row of the tree put down over a group, which opens the file there.
  ///
  /// The same two landings a tab has, answered the way the two above answer
  /// them: a place along a bar or a centre opens it in that group, and each of
  /// the four edges splits the group there and opens it alone in the new half.
  /// A release away from every group never reaches here, so there is nothing
  /// here that does nothing — which is what leaves the tree untouched by a drag
  /// that came to nothing.
  const bring = (path: string, where: Landing): void => {
    if ("at" in where) {
      show(path, where.group, where.at);

      return;
    }

    if (where.zone === "centre") {
      // At the end of the bar, there being no place along one in this gesture:
      // what the hand pointed at was the group.
      show(path, where.group, where.group.tabs().length);

      return;
    }

    const tab: Tab = { file: path };
    const made = fresh();
    const way: Way =
      where.zone === "left" || where.zone === "right" ? "beside" : "below";

    // Whether anything is reading this file already, asked before the tab joins
    // a group: a second view of a file somebody has typed into and not saved
    // would otherwise read the disk over their text.
    const anywhere = views(path) > 0;

    batch(() => {
      made.setTabs([tab]);
      made.setChosen(keyed(tab));

      setLayout((was) =>
        split(
          was,
          where.group.id,
          way,
          made,
          where.zone === "left" || where.zone === "above",
        ),
      );

      setActive(made.id);
    });

    if (!anywhere) {
      void reread(path);
    }
  };

  /// Where a pointer at this point would put a tab: a place along the bar it is
  /// over, a zone of the content it is over, or nothing at all where it is over
  /// neither.
  ///
  /// Asked of the groups as they are drawn rather than worked out from the
  /// tree, for the reason the sidebar asks its own rows where they are: how
  /// wide a tab stands is a name's length and a browser's font, and a drag that
  /// guessed would mark a place the hand is not pointing at.
  const landing = (x: number, y: number): Landing | null => {
    for (const box of layer.querySelectorAll<HTMLElement>(`.${styles.group}`)) {
      const group = groups().find(
        (one) => one.id === Number(box.dataset.group),
      );

      if (group === undefined) {
        continue;
      }

      const bar = box.querySelector<HTMLElement>(`.${styles.tabs}`);

      if (bar !== null && inside(bar.getBoundingClientRect(), x, y)) {
        // The first tab the point falls in front of — a tab's own middle being
        // where one place along the bar becomes the next — and the end of the
        // bar where it falls past every one of them.
        const frames = [
          ...bar.querySelectorAll<HTMLElement>(`.${styles.tabFrame}`),
        ];
        const at = frames.findIndex((frame) => {
          const its = frame.getBoundingClientRect();

          return x < its.left + its.width / 2;
        });

        return { group, at: at < 0 ? frames.length : at };
      }

      const room = box.querySelector<HTMLElement>(`.${styles.room}`);
      const over = room?.getBoundingClientRect();

      if (over !== undefined && inside(over, x, y)) {
        return { group, zone: zoned(over, x, y) };
      }
    }

    return null;
  };

  /// Where the line goes on a tab, where it goes on this one at all: in front
  /// of it, or after it where it is the last on its bar and the tab in the hand
  /// would land past every one of them.
  ///
  /// On the tabs rather than between them, there being no between: a bar is a
  /// row of tabs that abut, and what says where one would land is a mark on the
  /// one it would land beside.
  const lined = (group: Group, at: number): "before" | "after" | undefined => {
    const where = mark();

    if (where === null || where.group !== group || !("at" in where)) {
      return undefined;
    }

    if (where.at === at) {
      return "before";
    }

    return where.at === group.tabs().length && at === where.at - 1
      ? "after"
      : undefined;
  };

  /// And which of a group's five zones the hand is over, where it is over that
  /// group's content at all.
  ///
  /// What the band is drawn from, and the whole of what says which band: one
  /// landing at a time across the pane, so one group of the pane draws one of
  /// these and the rest draw none.
  const banded = (group: Group): Zone | undefined => {
    const where = mark();

    return where !== null && "zone" in where && where.group === group
      ? where.zone
      : undefined;
  };

  /// A press that let go about where it landed is a press, and a press turns to
  /// the tab. One that carried it is not: the tab is where they put it, and
  /// turning to it as well would be answering one gesture twice.
  const turn = (group: Group, tab: Tab): void => {
    if (carrying !== null && "tab" in carrying && carrying.tab === tab) {
      carrying = null;

      return;
    }

    setActive(group.id);
    group.setChosen(keyed(tab));
  };

  // A pane that goes away mid-drag takes the whole drag with it: the listeners
  // it hung on the window, and its refusal of the scroll. Nothing else would
  // ever take those off again — what would have is a drop that is never coming.
  onCleanup(() => {
    stop?.();
    document.removeEventListener("touchmove", refuseScroll);
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

    return openTerminal(device(), props.conversation.id)
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
  /// socket to hear it on, so it simply goes — **out of every group holding
  /// it**, the way a shell that ended goes and the way **New terminal**
  /// replacing one takes it. A sentence about a shell that never started is one
  /// shell's however many views of it a split made, and a copy left behind
  /// would be a tab with its sentence taken away, no socket to close and
  /// nothing at the server for its × to end. Nothing is drawn about a request
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
      everywhere((was) =>
        was.filter((one) => !("terminal" in one && one.terminal === number)),
      );
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

  /// Every open tab follows its file, which is what a rename in the tree leaves
  /// this pane to do.
  ///
  /// **The path is written down in more places than the tab.** What each group
  /// is showing and what it has open; what each of those was read as; the bar a
  /// refused save left standing; which of them a save is in flight for; which
  /// folders of the tree are open, and the rows each of them last read; the text
  /// this device is holding for a file nobody has saved; and the buffer itself.
  /// Every one of them is keyed by path, and a rename moves the key.
  ///
  /// **A folder carries everything under it**, which is why what is moved is a
  /// prefix rather than one string: a folder renamed is every open tab whose
  /// path was inside it and every folder of the tree expanded beneath it, all
  /// moved in the one press (ADR 0019, *The tree*). A file is the same rule with
  /// nothing under it.
  ///
  /// **And the buffer is re-made rather than moved.** A Monaco model is
  /// registered at the file's own address and the package cannot rename one, so
  /// the model is disposed and made again with the same text in it. What carries
  /// over is the text and — because the reading moved with it — whether the tab
  /// is dirty; what goes is the undo stack, which is Monaco's limit rather than a
  /// choice. The same file open in two groups is one buffer under both, so both
  /// tabs follow together.
  const renamed = (from: string, to: string): void => {
    // What the rename reached: the path itself, and anything under it where it
    // was a folder. Spelled with either separator, because a path here is
    // spelled the way the root it came from is and a Worktree on Windows uses a
    // `\` — see `FolderEntry::path`, which is where the join is made.
    const moved = (path: string): string | null => {
      if (path === from) {
        return to;
      }

      const next = path.slice(from.length, from.length + 1);

      return path.startsWith(from) && (next === "/" || next === "\\")
        ? to + path.slice(from.length)
        : null;
    };

    // Taken before anything moves: the tabs are about to be re-keyed, and what
    // each buffer is re-made from is the text it is holding now.
    const held = buffers();
    const carried = [
      ...new Set(viewed().flatMap((tab) => ("file" in tab ? [tab.file] : []))),
    ].flatMap((path) => {
      const next = moved(path);

      return next === null ? [] : [[path, next] as const];
    });

    batch(() => {
      // The tabs of every group, and which of them each group is showing. The
      // tab it was turned to is found before the move and named again after it,
      // rather than its key being taken apart: what a tab is known by is
      // [`keyed`]'s business.
      for (const group of groups()) {
        const was = group.tabs();
        const now = was.map((tab) =>
          "file" in tab ? { file: moved(tab.file) ?? tab.file } : tab,
        );
        const at = was.findIndex((tab) => keyed(tab) === group.chosen());

        group.setTabs(now);

        if (at >= 0) {
          group.setChosen(keyed(now[at]!));
        }
      }

      // What each was read as, with the path inside the reading moved too: a
      // reading that went on naming where the file *was* would be the one thing
      // here still saying so.
      setReadings((was) => rekeyed(was, moved, atPath));
      setBars((was) => rekeyed(was, moved));

      for (const path of [...saving]) {
        const next = moved(path);

        if (next !== null) {
          saving.delete(path);
          saving.add(next);
        }
      }

      // And the folders of the tree that are open, listings and all — the rows
      // one holds name their own paths, and a row still naming the old one is a
      // press that would open a file that is not there.
      setExpanded((was) =>
        rekeyed(was, moved, (listing) => listed(listing, moved)),
      );
    });

    for (const [path, next] of carried) {
      // What this device is holding for it goes first, `release` being what
      // throws that away: a rename in the seconds after a reload is a tab whose
      // read has not landed, and the text it is going to put in the buffer is
      // still on the device.
      carry(path, next);

      const buffer = held[path];

      release(path);

      if (buffer !== undefined) {
        hold(next, buffer.text());
      }
    }
  };

  /// And a file taken away out of the tree keeps its tab, which is the other
  /// half of a tab following its file.
  ///
  /// **The tab stays, read-only, saying the file is gone**, with whatever was in
  /// it still there to be read and copied out (ADR 0019, *The tree*): a tab that
  /// vanished under somebody with unsaved text would take the text with it, and
  /// the × is the one thing that closes it — asking first, the text being text
  /// the disk has not got.
  ///
  /// So what moves is the *reading* and nothing else: it becomes the answer a
  /// read of that path would give now, and the buffer under it is left exactly
  /// where it is. Which is also what makes the tab dirty, and so what this device
  /// writes down — see `keeping.ts`, where gone and dirty are the one comparison.
  ///
  /// **A folder carries everything under it**, on the prefix [`renamed`] moves
  /// on and for its reason: one press took the whole of it, so every tab inside
  /// it and every folder of the tree expanded beneath it goes the same way.
  const deleted = (path: string): void => {
    // What the delete reached: the path itself, and anything under it where it
    // was a folder. Spelled with either separator, for the reason [`renamed`]'s
    // is — a path here is spelled the way the root it came from is.
    const gone = (one: string): boolean => {
      if (one === path) {
        return true;
      }

      const next = one.slice(path.length, path.length + 1);

      return one.startsWith(path) && (next === "/" || next === "\\");
    };

    batch(() => {
      // Every open file that has gone reads as missing, which is what its tab
      // draws the line over its text from — and is what a read of that path
      // would answer now, so a reload comes back to the same tab.
      setReadings((was) =>
        Object.fromEntries(
          Object.entries(was).map(([at, read]) => [
            at,
            gone(at) ? "Missing" : read,
          ]),
        ),
      );

      // And whatever the last save left standing goes with it: a bar is a
      // question about a file on the disk, and there is no longer one.
      setBars((was) =>
        Object.fromEntries(Object.entries(was).filter(([at]) => !gone(at))),
      );

      // And the folders of the tree that were open under it, listings and all: a
      // folder kept here is a folder the tree draws without reading again, and
      // one that is not there would be a column of rows that are not either.
      setExpanded((was) =>
        Object.fromEntries(Object.entries(was).filter(([at]) => !gone(at))),
      );
    });
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
  ///
  /// **And `restored` is the text this device was holding when the page went**,
  /// where it was holding any: it goes into the buffer in place of what came
  /// back, so the tab is dirty against the version the file reads at *now*. A
  /// file the agent rewrote overnight comes back to the human's text over the
  /// disk's own, which is what makes the comparison — and the save that names
  /// that version — true of what is really there.
  const reread = (
    path: string,
    keeping = false,
    restored?: string,
  ): Promise<void> => {
    // Whatever the last save said goes with the reading it was about: the bar
    // is a question about the disk, and this is the disk answering.
    unbar(path);

    return readFile(device(), props.conversation.id, path)
      .then((reading) => {
        if (keeping) {
          setReadings((was) => ({ ...was, [path]: reading }));
          return;
        }

        landed(path, reading, restored);
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

  /// What a read that came back comes to: the reading where the tab draws it
  /// from, and its text where the buffer is.
  ///
  /// Apart from [`reread`] because the read is not always the one just made —
  /// the disk moving under the pane is a read of its own, and what it does with
  /// what comes back is this same thing (see [`again`]).
  const landed = (
    path: string,
    reading: FileReading,
    restored?: string,
  ): void => {
    setReadings((was) => ({ ...was, [path]: reading }));

    if (typeof reading !== "string" && "Text" in reading) {
      // The buffer, made out of what was read where the file has none yet and
      // written with it where it has — which is the whole of the difference
      // between opening a file and **Reload**. Or out of the text the device
      // came back holding, where this is the read that restored the tab.
      hold(path, restored ?? reading.Text.text);
    } else if (reading === "Missing") {
      // A file that is gone keeps whatever text there is, which is the whole
      // of what its tab is for: what this device came back holding goes into
      // the buffer, and a buffer that is already here is left exactly as it
      // is. Letting go here would be the one read that threw the last copy
      // of somebody's text away.
      if (restored !== undefined) {
        hold(path, restored);
      }
    } else {
      release(path);
    }
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

  /// And read one again because the disk moved, rather than because somebody
  /// asked for it.
  ///
  /// The same one request the tree makes of its folders, and what is done with
  /// the answer is the buffer's own (ADR 0019, *Following the disk*):
  ///
  /// - **A file reading at the version it already did is left alone entirely**,
  ///   which is nearly every file on nearly every Nudge — a build rewrote
  ///   another folder, a timestamp moved, the file was written with the bytes it
  ///   already held. Nothing is set, so nothing is drawn again.
  /// - **A clean buffer takes the new text silently**, keeping its caret as near
  ///   as it can — see `written` in `keeping.ts`, which writes the part that
  ///   moved rather than the file. Every view of it shows it, there being one
  ///   buffer under them, and there is no bar: nothing was asked and nothing was
  ///   lost.
  /// - **A dirty one keeps the human's text and raises the bar**, which is the
  ///   same **Reload** / **Keep mine** the refused save raises — drawn now the
  ///   moment the disk moves rather than at the next Ctrl+S. The reading is left
  ///   where it is with it, so a save made under the bar is still a write over
  ///   the version this tab was read at and is still refused.
  /// - **And a file that has gone keeps its tab** whether or not it is dirty,
  ///   the way one deleted out of the tree does: the text in it is the only copy
  ///   there is, and a bar asking which text the next save is of would be a
  ///   question about a file there is nothing to save to.
  ///
  /// **A save in flight is not read at all.** The write is about to answer with
  /// the version it made — and the Nudge that write itself raises is the disk
  /// moving under nobody. A read that crossed it would be this pane finding its
  /// own text on the disk and putting a bar up over the human's typing.
  ///
  /// And a tab that moved while the read was out is left where it is: it was
  /// closed, renamed, reloaded by a press, or saved, and what came back is about
  /// a file the pane is no longer standing on. A request that never landed is no
  /// news either — a connection that dropped under a tab nobody touched is not a
  /// reason to draw a sentence over somebody's text.
  const again = (path: string): Promise<void> => {
    const was = readings()[path];

    if (was === undefined || saving.has(path)) {
      return Promise.resolve();
    }

    return readFile(device(), props.conversation.id, path)
      .then((reading) => {
        if (readings()[path] !== was || saving.has(path) || same(was, reading)) {
          return;
        }

        if (reading !== "Missing" && dirty(path)) {
          // Where one is already up, left standing rather than raised again:
          // the bar is the same question, and putting it back would redraw it
          // under the hand that is about to press it.
          setBars((had) =>
            had[path] === "moved" ? had : { ...had, [path]: "moved" },
          );

          return;
        }

        // And whatever the last save left standing goes with the reading it was
        // about, the way a read somebody pressed for takes it down: the bar is
        // a question about the disk, and this is the disk answering.
        unbar(path);
        landed(path, reading);
      })
      .catch(() => {});
  };

  /// The whole of what a `files` Nudge comes to in the tabs: every file the
  /// pane has open, read again.
  ///
  /// One read per open file and none for anything else — the tree beside it
  /// re-reads the folders it has expanded, which is its own subscription (see
  /// `Tree.tsx`). A pane with nothing open reads nothing at all, which is what
  /// a Nudge about a Worktree nobody has a tab in costs.
  const follow = (): void => {
    for (const path of Object.keys(readings())) {
      void again(path);
    }
  };

  // And what brings that news: the watcher the socket above runs, saying this
  // Conversation's Worktrees moved (ADR 0019, *Following the disk*).
  //
  // A subscription of the pane's own rather than a row of the table in
  // `nudge.ts`, for the tree's reason: that table invalidates queries, and the
  // readings are held above this pane with the tabs and the text nobody has
  // saved. Let go of with the pane, so one that is not drawn reads nothing.
  createEffect(() => {
    onCleanup(whenFilesMove(device(), props.conversation.id, follow));
  });

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

    return writeFile(device(), props.conversation.id, path, read.version, text)
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
  /// human was last working in is the one they meant. A row *dragged* out of
  /// the tree names a group of its own, which is [`bring`]; what both of them
  /// do once they have one is [`show`] below.
  const openFile = (path: string): void => {
    // One that carried this row somewhere is not a press: the file is open
    // where they put it, and opening it here as well would be answering one
    // gesture twice.
    if (carrying !== null && "path" in carrying && carrying.path === path) {
      carrying = null;

      return;
    }

    const group = into();

    show(path, group, group.tabs().length);
  };

  /// And the opening itself, into a named group at a named place along its bar
  /// — which is what a press does of the active group, and what a row dragged
  /// out of the tree does of the group it was let go over.
  ///
  /// The group becomes the active one. A press has only said so already, that
  /// group being the active one to begin with; a drop is the work going where
  /// the hand pointed, which is what active means.
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
  const show = (path: string, group: Group, at: number): void => {
    const tab: Tab = { file: path };
    const anywhere = views(path) > 0;

    batch(() => {
      group.setChosen(keyed(tab));
      setActive(group.id);

      // Already open in this group: a tab to turn to rather than a second tab
      // beside the first, there being nothing a second view of a file in the
      // one group could show that the first is not showing already.
      if (!group.tabs().some((one) => keyed(one) === keyed(tab))) {
        group.setTabs((was) => [...was.slice(0, at), tab, ...was.slice(at)]);
      }
    });

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
    closeTerminal(device(), props.conversation.id, tab, asked)
      .then((outcome) => {
        setAsking(outcome === "Busy" ? tab : undefined);
      })
      // A request that never landed leaves the tab where it is, for the reason
      // nothing is drawn about one: the shell is the server's, and a tab still
      // there is what says it is still running.
      .catch(() => setAsking(undefined));

  /// Turn the active group to the tab after the one it is showing, or to the
  /// one before it — Ctrl+PageDown and Ctrl+PageUp, which wrap at each end.
  ///
  /// A group with nothing open has nowhere to go, and one with a single tab is
  /// already where it is going: both fall out of the arithmetic rather than
  /// being asked about.
  const step = (by: number): void => {
    const group = into();
    const open = group.tabs();
    const at = open.findIndex((one) => keyed(one) === showing(group));

    if (at < 0) {
      return;
    }

    group.setChosen(keyed(open[(at + by + open.length) % open.length]!));
  };

  /// Split a group beside itself, showing whatever it is showing — what the
  /// icon at the end of its bar does, and what Ctrl+\ does.
  ///
  /// Nothing at all where the group has nothing open: a split is this group
  /// again, and there is no second view to be had of a tab that is not there.
  const beside = (group: Group): void => {
    const shown = group.tabs().find((one) => keyed(one) === showing(group));

    if (shown !== undefined) {
      divide(group, shown, "beside");
    }
  };

  /// Whether the quick-open palette is up.
  ///
  /// The pane's own rather than the keeping's, unlike the tabs and the tree
  /// beside them: a palette is a press being made rather than something that is
  /// open, so a pane swapped away for an Event and back comes back without one —
  /// which is what closing it having opened nothing already means.
  const [quick, setQuick] = createSignal(false);

  /// What this pane takes from the keyboard: Ctrl+S, and the four keystrokes a
  /// browser leaves to the page.
  ///
  /// Ctrl+S — Cmd+S on a Mac — is the whole of how a file is saved. Saving is
  /// explicit, which is VS Code's default and the one the bar over a refused
  /// save depends on: an autosave has no dirty state to hold the human's text
  /// in while they decide what to do about a collision (ADR 0019, *Versioned
  /// reads, and a stale write is refused*). Taken from the browser first, its
  /// own Save Page being nothing anybody meant.
  ///
  /// Ctrl+PageDown and Ctrl+PageUp walk the active group's tabs and wrap at
  /// each end, Ctrl+\ splits it beside itself the way its bar's icon does, and
  /// Ctrl+` opens a shell in it the way **New terminal** does, and Ctrl+P drops
  /// the quick-open palette over the pane — VS Code's own key, taking the
  /// browser's Print with it, which is still on the browser's own menu where
  /// anybody who meant it would look. Ctrl+W and
  /// Ctrl+Tab are the window's own and are deliberately not among them: a page
  /// that swallowed either would be a pane fighting the browser around it.
  ///
  /// On the document rather than on the editor or on any one group. Monaco
  /// binds nothing to Ctrl+S itself, and a terminal is a grid with the focus in
  /// it — so a press arrives here whether the hands were in a file, in a shell
  /// beside it or on the tree, and what each of these acts on is the **active**
  /// group rather than whatever happens to have the focus. Which is also why
  /// not one of them is a key the grid already had: Ctrl+C interrupts, Ctrl+D
  /// ends and Ctrl+L clears, and every one of those goes up the socket
  /// untouched.
  ///
  /// Only while this pane is mounted, which is the whole reach of the listener:
  /// Code is the only thing in this workbench with tabs to walk or a file in it
  /// to write.
  const pressed = (event: KeyboardEvent): void => {
    if (!(event.ctrlKey || event.metaKey) || event.altKey) {
      return;
    }

    switch (event.key) {
      case "s":
      case "S": {
        const group = into();
        const open = group.tabs().find((one) => keyed(one) === showing(group));

        // A group showing a shell has nothing to write, and the press is left
        // where it would have gone without this pane.
        if (open === undefined || !("file" in open)) {
          return;
        }

        event.preventDefault();
        void save(open.file);
        return;
      }

      case "p":
      case "P":
        event.preventDefault();
        setQuick(true);
        return;

      case "PageDown":
      case "PageUp":
        event.preventDefault();
        step(event.key === "PageDown" ? 1 : -1);
        return;

      case "\\":
        event.preventDefault();
        beside(into());
        return;

      case "`":
        event.preventDefault();
        void open();
        return;

      default:
        return;
    }
  };

  document.addEventListener("keydown", pressed);
  onCleanup(() => document.removeEventListener("keydown", pressed));

  /// The files the device came back holding, read now.
  ///
  /// A reload restores the tabs and the layout they stand in, and nothing
  /// behind them: what a file says is the disk's to answer, so every restored
  /// tab is read here the way a file pressed in the tree is. The text the human
  /// had typed and not saved goes over the top of what comes back, so it is
  /// dirty against the version the file reads at *now* — which is what puts the
  /// bar on a file the agent rewrote overnight, rather than a save going out
  /// over a version that is no longer there (ADR 0019, *Versioned reads, and a
  /// stale write is refused*).
  ///
  /// **A file with a reading already is left alone**, which is what tells a
  /// reload from a swap: a pane swapped for an Event and back finds its
  /// readings where it left them, and reading them again would be a fresh
  /// version under text the human never saw.
  ///
  /// Once per mount, and there is nothing to do on nearly all of them: a pane
  /// that opened on a Conversation this device has never had Code open in has
  /// no tabs to restore.
  onMount(() => {
    const open = new Set(
      viewed().flatMap((tab) => ("file" in tab ? [tab.file] : [])),
    );

    for (const path of open) {
      if (readings()[path] === undefined) {
        void reread(path, false, recall(path));
      }
    }
  });

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
          {/* The pane's own ⋯, which is where what is about the pane rather
              than about anything open in it goes: the three settings every
              editor is drawn with (ADR 0019, *Monaco, whole*).

              Here rather than on a settings page somewhere else, which would
              be a page about one pane of one Conversation — and here rather
              than per editor, because one answer for the whole pane is what
              makes a press reach every group at once. */}
          <Menu
            class={styles.paneActions!}
            label="Editor settings"
            name="Editor settings"
            mark
          >
            {() => (
              <>
                {/* Left open on a press, unlike every other menu in the app:
                    these are settings rather than things to do, the tick beside
                    each is the answer to what the press asked, and somebody
                    turning the wrap on is as likely as not about to reach for
                    the size under it. */}
                <Setting
                  says="Word wrap"
                  class={styles.wrap!}
                  on={drawing().wrap}
                  press={() => draw({ wrap: !drawing().wrap })}
                />

                {/* The one of the three that is not a switch, so it is a level
                    of this same menu rather than a row: one card, one backdrop
                    and one way out, with the sizes inside it. */}
                <Nested label="Font size">
                  {() => (
                    <For each={SIZES}>
                      {(size) => (
                        <button
                          type="button"
                          role="menuitemradio"
                          class={`${styles.setting!} ${styles.size!}`}
                          aria-checked={drawing().size === size}
                          onClick={() => draw({ size })}
                        >
                          {size}px
                          <Show when={drawing().size === size}>
                            <span aria-hidden="true">✓</span>
                          </Show>
                        </button>
                      )}
                    </For>
                  )}
                </Nested>

                <Setting
                  says="Minimap"
                  class={styles.minimap!}
                  on={drawing().minimap}
                  press={() => draw({ minimap: !drawing().minimap })}
                />
              </>
            )}
          </Menu>

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
          pick={pick}
          carried={taken}
          renamed={renamed}
          deleted={deleted}
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
          <div class={styles.stack} ref={layer}>
            <For each={groups()}>
              {(group) => (
                <div
                  class={styles.group}
                  style={stood(group.id)}
                  // Which group this box is, read by a drag asking which bar
                  // the pointer is over — a question about the page as it is
                  // drawn rather than about the tree behind it, the way the
                  // sidebar's rows carry the id a drag reads off them.
                  data-group={group.id}
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
                        {(tab, at) => (
                          // Two buttons rather than one: the tab is pressed to
                          // show what it holds and the × is pressed to close
                          // it, and a button inside a button is not a thing a
                          // browser draws. The frame around them is the tab as
                          // the eye reads it, and is what takes the fill of the
                          // one showing.
                          <div
                            class={styles.tabFrame}
                            // Where the tab in the hand would land if it were
                            // let go of now: the line VS Code draws between two
                            // tabs, drawn on the tab it would go beside rather
                            // than standing between two of them — a line with a
                            // width of its own would move the bar under the
                            // very hand it is answering.
                            data-mark={lined(group, at())}
                            classList={{ [styles.lifted!]: carried() === tab }}
                          >
                            <button
                              type="button"
                              class={styles.tab}
                              aria-pressed={showing(group) === keyed(tab)}
                              // The press, which is a press that turns to the
                              // tab, a scroll of the bar or a drag — settled by
                              // what the hand does next, and watched at the
                              // window from here (ADR 0019, *Tabs and groups*).
                              // Which hand it is is read here too, for the menu
                              // below: a long press is how a tab is picked up,
                              // so the menu is the mouse's alone.
                              onPointerDown={(event) => grab(event, group, tab)}
                              onContextMenu={(event) => ask(event, group, tab)}
                              onClick={() => turn(group, tab)}
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
                        press={() => beside(group)}
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

                  {/* And the group with nothing in it, which is where a
                      Conversation with no shells running lands and where the
                      last of them leaves it. Drawn where a tab's content goes
                      rather than over the whole pane: the tree stands beside
                      this, and a pane-wide notice would be a sentence over that
                      too.

                      A list that would not read settles it as well as one that
                      did. The hint waits on somebody having looked, and a read
                      that ended in the line above is a look that is over —
                      without this the pane would sit on *Reading this
                      conversation's terminals…* under a sentence saying it
                      could not, with no way to open one and try again. */}
                  <Show when={group.tabs().length === 0}>
                    <Show
                      when={read() || terminals.isError}
                      fallback={
                        <Empty>Reading this conversation's terminals…</Empty>
                      }
                    >
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
                    </Show>
                  </Show>

                  {/* The room this group's views stand in, which is also the
                      drop target with five zones in it: the box a drag is
                      measured against, and the box the band it would take is
                      drawn over. */}
                  <div class={styles.room}>
                    {/* Where what this group has open is drawn: a slot rather
                        than the views themselves, because a view is drawn once
                        for the whole pane and *put* into whichever group holds
                        it — see the `For` below the groups. One box per group
                        and one for the life of the group, so that a tab
                        arriving in it has somewhere to be put.

                        Nothing of this page's is ever drawn inside it, which is
                        what the empty state above and the band below are doing
                        outside it: a box whose children a framework is keeping
                        is a box it empties when those children go, and it would
                        take the views placed in it along with them. */}
                    <div
                      class={styles.content}
                      ref={(box) => {
                        setSlots((was) => ({ ...was, [group.id]: box }));
                        onCleanup(() =>
                          setSlots((was) => {
                            const rest = { ...was };

                            delete rest[group.id];

                            return rest;
                          }),
                        );
                      }}
                    />

                    {/* And the zone under the hand, where the hand is over this
                        group's content: what a release would do, drawn before
                        it is done. The centre is the whole of the room, that
                        being the tab moving in; an edge is the half the new
                        group would take, that being the split it would make.

                        One at a time across the whole pane — a pointer is in
                        one zone of one group — and none at all every moment
                        nobody is carrying a tab. */}
                    <Show when={banded(group)}>
                      {(zone) => <div class={styles.zone} data-zone={zone()} />}
                    </Show>
                  </div>
                </div>
              )}
            </For>

            {/* And every view there is, drawn once for the whole pane and put
                into the slot of whichever group holds its tab.

                Which is what lets a tab be dragged from one group to another
                without being made again: a terminal's box carries a live socket
                and the window's own memory of what has scrolled past it, and a
                file's carries a caret and an undo stack, and a view drawn
                inside the group holding it would lose every one of those the
                moment the tab moved. So the box is made once, against the tab
                that is the view — see [`copied`] — and moved between groups the
                way the groups themselves are placed rather than nested.

                Nothing is drawn where this stands, the box going into a group
                rather than here; and it goes back out again when the tab
                closes, which is what takes the socket down. */}
            <For each={viewed()}>
              {(tab) => {
                /// Which group is holding it, which is the whole of where the
                /// box goes and of whether it is the one showing.
                const whose = createMemo(() =>
                  groups().find((one) => one.tabs().includes(tab)),
                );
                const shown = (): boolean => {
                  const group = whose();

                  return group !== undefined && showing(group) === keyed(tab);
                };

                const box = (
                  <div class={styles.view}>
                    {"file" in tab ? (
                      // A file's tab is drawn whether or not it is the one
                      // showing, and hidden when it is not — the way a
                      // terminal's is, and for the near reason: a grid taken
                      // down is a shell nobody could come back to, and an
                      // editor taken down is a caret and an undo stack nobody
                      // can come back to. The text itself was never at risk,
                      // the buffer being above this pane; what the hiding keeps
                      // is where the human was in it.
                      //
                      // One of these per *view*: the same file in two groups is
                      // two of these over the one buffer, which is what makes
                      // them type together.
                      <Opened
                        showing={shown()}
                        reading={readings()[tab.file]}
                        buffer={buffers()[tab.file]}
                        name={named(tab.file)}
                        drawn={drawing()}
                        bar={bars()[tab.file]}
                        reload={() => void reread(tab.file)}
                        keep={() => void reread(tab.file, true)}
                      />
                    ) : tab.terminal > 0 ? (
                      <Attached
                        at={terminalSocket(
                          device(),
                          props.conversation.id,
                          tab.terminal,
                        )}
                        showing={shown()}
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
                      // A tab the server never opened a shell for has no grid
                      // to stand under the sentence, and nothing to attach to:
                      // the refusal is the whole of it.
                      <Show when={shown()}>
                        <ErrorLine>{over()[tab.terminal]}</ErrorLine>
                      </Show>
                    )}
                  </div>
                ) as HTMLElement;

                // Put where its group is, and moved when that changes — which
                // is a box moving rather than anything being made or taken
                // down. A group with no slot yet is a group in the middle of
                // being drawn, and the next run of this puts the box in it.
                createEffect(() => {
                  const slot = slots()[whose()?.id ?? -1];

                  if (slot !== undefined && box.parentNode !== slot) {
                    slot.append(box);
                  }
                });

                onCleanup(() => box.remove());

                // And nothing where the list itself stands: the box belongs to
                // a group rather than to here, and this is only where it was
                // made.
                return undefined;
              }}
            </For>

            {/* And a divider on every border there is — one per split, drawn
                over the line where its two halves meet.

                A separator rather than a button, because what it does to the
                page is a value rather than an action: the share of the split
                the half before it is worth. Which is what it carries, and what
                the arrow keys move it by for the pointer nobody dragging with
                a keyboard has — along its own split's axis, so a border down
                the middle answers left and right and one across answers up and
                down. The frame's own divider is the pattern, in `Panes.tsx`.

                `Index` rather than `For`: a drag rewrites the border it is
                dragging on every pointer move, and a list reconciled by value
                would take the handle down and make it again under the pointer
                — and under the keyboard's focus, which is a nudge that can
                only be made once. How many borders there are changes when the
                tree is reshaped and at no other time, which is exactly what
                indexing keys on. */}
            <Index each={bordering()}>
              {(border) => (
                <div
                  class={styles.divider}
                  data-way={border().way}
                  role="separator"
                  aria-orientation={
                    border().way === "beside" ? "vertical" : "horizontal"
                  }
                  aria-label={
                    border().way === "beside"
                      ? "Resize what is left of this border"
                      : "Resize what is above this border"
                  }
                  aria-valuenow={Math.round(border().share)}
                  aria-valuemin={Math.round(border().least)}
                  aria-valuemax={Math.round(border().most)}
                  tabindex="0"
                  style={along(border())}
                  onPointerDown={(event) => drag(border(), event)}
                  onKeyDown={(event) => nudge(border(), event)}
                />
              )}
            </Index>
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

      {/* And the quick-open palette, while Ctrl+P has one up: a field over every
          root's files, matched on the page, opening the pick into the active
          group — see [`./Quick`], which is where the list is read.

          Drawn only while it is open rather than told whether it is, unlike the
          two cards above: what makes its list fresh is that it is read when the
          palette opens, and a component mounted when the palette opens is what
          makes those the same moment. */}
      <Show when={quick()}>
        <Quick
          conversation={props.conversation.id}
          open={openFile}
          close={() => setQuick(false)}
        />
      </Show>
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

/// One of the pane's settings that is on or off, as a row of its menu.
///
/// A `menuitemcheckbox` rather than a button, because that is what it is: the
/// press does not do something and leave, it moves a setting whose state is
/// part of the row. Which is also why the tick is drawn — `aria-checked` says
/// it to a screen reader, and a mark beside the words says it to everybody
/// else.
///
/// Two of the three are this shape. The third is a number and is a level of the
/// same menu, written where it is used.
function Setting(props: {
  /// What the row reads as, which is what the setting is called.
  says: string;
  /// The caller's class on it, so each of them can be found.
  class: string;
  on: boolean;
  press: () => void;
}): JSX.Element {
  return (
    <button
      type="button"
      role="menuitemcheckbox"
      class={`${styles.setting} ${props.class}`}
      aria-checked={props.on}
      onClick={() => props.press()}
    >
      {props.says}
      <Show when={props.on}>
        <span aria-hidden="true">✓</span>
      </Show>
    </button>
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
  /// How the pane draws its editors, which is the pane's own answer rather than
  /// this tab's: it is handed through to whichever editor this tab holds, and a
  /// change on the menu above reaches every one of them at once.
  drawn: Drawn;
  /// What the last save came to, where it came to anything to draw.
  bar: Bar | undefined;
  /// **Reload**: take what is on the disk now, text and version together.
  reload: () => void;
  /// And **Keep mine**: read the disk for its version, and keep the text that
  /// is here over it, so that the next save lands.
  keep: () => void;
}): JSX.Element {
  /// Whether the file is gone from the disk with its text still here, which is
  /// what a tab left standing over a deleted file is.
  ///
  /// The buffer is what tells this from the refusal beside it: `Missing` with
  /// nothing behind it is a file somebody tried to open and could not, and there
  /// is nothing to draw; `Missing` over a buffer is a file that *was* open, and
  /// the text in it is now the only copy there is (ADR 0019, *The tree*).
  ///
  /// Which is also what a reload comes back to, the read of a restored tab
  /// answering `Missing` and the device handing over the text it kept.
  const gone = (): boolean =>
    props.reading === "Missing" && props.buffer !== undefined;

  /// The refusal this came back as, where it came back as one — the server's
  /// own sentence for the unreadable, which is the only one of them that says
  /// something this side could not have worked out.
  const why = (): string | null => {
    const read = props.reading;

    if (read === undefined || gone()) {
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

      {/* And the line over a file that has gone, in the bar's own place: what is
          under it is the text as it was, and what it says is that this is now
          the only copy of it. A statement rather than a question — there is
          nothing to choose between, the file being gone either way. */}
      <Show when={gone()}>
        <p class={styles.gone} role="status">
          {GONE}
        </p>
      </Show>

      <Switch fallback={<Empty>Opening this file…</Empty>}>
        {/* Before the refusal below, which this would otherwise be one of: a
            file that is gone with its text still here is a tab to read rather
            than a sentence about a file that could not be opened. */}
        <Match when={gone()}>
          <Editor
            name={props.name}
            model={props.buffer?.model}
            writable={false}
            drawn={props.drawn}
          />
        </Match>
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
              drawn={props.drawn}
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

/// The bar a file that moved under unsaved text puts up: what happened, and the
/// two things to do about it.
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
/// while it stands — and a file that moved under somebody's typing is a thing to
/// come back to rather than a question to get out of the way.
///
/// **Raised the moment the disk moves**, which is what the watcher is for: the
/// pane reads every open file on a `files` Nudge, and a dirty one whose version
/// has changed is this (ADR 0019, *Following the disk*). A refused save raises
/// the same bar — which is the Nudge that never arrived, and the one moment the
/// collision cannot be missed.
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

/// How far a pointer may travel and still have been a press, in pixels.
///
/// A press is not a steady thing — a mouse moves a pixel or two between going
/// down and coming up — so a tab that began moving at the first move would be a
/// tab that could not be pressed at all. The sidebar's cards are held off by
/// the same distance, in `Conversations.tsx`.
const GRACE = 5;

/// And how long a finger holds a tab still before it lifts, in milliseconds.
///
/// Long enough that a swipe along a bar of tabs is never taken for it, short
/// enough that holding one is not waiting for it — the sidebar's again, and a
/// gesture the human meets in two places should ask the same of them in both.
const LIFT = 400;

/// How much of a group's content each of its four edges takes, as a fraction of
/// the way across it.
///
/// A quarter apiece leaves the middle half of the content in both directions as
/// the centre, which is what a hand aiming at the middle of a group hits and
/// what a hand aiming at a side has to mean. A corner belongs to whichever edge
/// it is nearer, there being no fifth thing to do with one.
///
/// What is *drawn* for an edge is the half the new group would take rather than
/// this — see `.zone` in the stylesheet: the band is what the drop would leave
/// behind, and a quarter drawn where a half is coming would be the page
/// answering with the wrong shape.
const EDGE = 0.25;

/// One of the five things a release over a group's content would do.
///
/// The centre moves the tab into that group, the way a release over its bar
/// does. Each edge splits the group there, with the tab alone in the new half
/// (ADR 0019, *Tabs and groups*).
type Zone = "centre" | "left" | "right" | "above" | "below";

/// And where a tab in the hand would land if it were let go now: a place along
/// some group's bar, or a zone of some group's content.
///
/// The two things a drag can be over, and what the pane marks while it is over
/// one — a line between two tabs, or the band a zone would take. One at a time
/// across the whole pane, a pointer being in one place.
type Landing = { group: Group; at: number } | { group: Group; zone: Zone };

/// And what a hand is carrying: a view out of the group it stands in, or a file
/// out of the tree.
///
/// The one gesture picks up either — the same grace, the same hold, the same
/// two landings — and what tells them apart is what a drop *does*: a view is
/// moved into place, and a path is opened there. Which is why the press holds
/// this rather than a tab: everything between the press and the release is the
/// same for both.
type Carrying = { group: Group; tab: Tab } | { path: string };

/// Whether a point is in a box, which is the whole of what a drop target is
/// asked.
function inside(box: DOMRect, x: number, y: number): boolean {
  return x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;
}

/// And which zone of a group's content a point in it is in: the edge it is
/// nearest where it is inside that edge's band, and the centre everywhere else.
///
/// Measured as fractions of the box rather than as lengths, so the bands of a
/// group half the pane wide and one a quarter of it are the same share of each
/// — an edge is a place on a group rather than a distance from a line.
///
/// A box with no size yet has no zones to speak of, and every fraction of it
/// would be infinite: it reads as the centre, which is the zone that does the
/// least.
function zoned(box: DOMRect, x: number, y: number): Zone {
  if (box.width <= 0 || box.height <= 0) {
    return "centre";
  }

  const edges: [Zone, number][] = [
    ["left", (x - box.left) / box.width],
    ["right", (box.right - x) / box.width],
    ["above", (y - box.top) / box.height],
    ["below", (box.bottom - y) / box.height],
  ];

  const [which, how] = edges.reduce((nearest, edge) =>
    edge[1] < nearest[1] ? edge : nearest,
  );

  return how < EDGE ? which : "centre";
}

/// What a tab being dragged does to the scroll under it: refuses it. Hung on
/// the document at the lift and taken off at the drop, so a finger scrolls a
/// bar of tabs every other moment of the day.
///
/// A function of its own rather than one made per drag, because removing a
/// listener means handing back the very same function.
const refuseScroll = (event: Event): void => event.preventDefault();
