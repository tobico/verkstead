//! The file tree down the side of Code: a root per worktree the conversation
//! has, and one folder read at a time under each
//! ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *The tree*).
//!
//! **A root is a worktree.** The conversation's own first, then each
//! companion's in the order the conversation carries them — and a read-only
//! companion is a root here, marked, rather than one left out: it is checked
//! out detached and there is nothing to commit in it, and there is plenty to
//! read. Which is also what bounds the files API: the server reads the
//! worktrees as itself, and a path under none of them is refused.
//!
//! **One folder at a time, read when it is expanded.** Nothing here walks, and
//! nothing is fetched until somebody presses a row — a tree that read itself
//! would be a recursive walk of a rust checkout's `target/` before the pane had
//! drawn. The shape a path field's browse already has, and for the same reason.
//!
//! **And read again on every expand.** A folder collapsed forgets what it held,
//! so opening it is a fresh reading of the disk. Nothing yet tells the page that
//! the disk moved — the agent writes, a build runs, a terminal tab checks out a
//! branch — so the collapse is the only moment there is to be sure by. Stage 04
//! of the roadmap is the watcher that makes that unnecessary; until then this is
//! what keeps the tree honest.
//!
//! **And which folders are open outlives the pane**, because the walk down to a
//! file is work the human did. The details pane is taken down whenever
//! something else is drawn in it, so what is open is held above the swap with
//! the tabs and the unsaved text rather than in this file — see `keeping.ts`.
//! A swap is not an expand, so what comes back is the listing that was last
//! read rather than a fresh one; the collapse is still where the tree is made
//! honest.
//!
//! **What git ignores is not in it**, and neither is `.git`. Both are the
//! server's doing — git is asked rather than reimplemented — so what arrives
//! here is already a listing with no `target/` in it.
//!
//! **A file pressed opens a tab.** The press is the whole of what the tree does
//! with a file: what it is handed is the path, and what happens to it is the
//! groups of tabs beside this — see `Code.tsx`, which is where a reading of one
//! is held, and where the active group is the one it opens in. The same file
//! pressed twice turns to the tab it already has.
//!
//! **And a file dragged out of it opens where it is dropped.** The gesture is
//! the one a tab is dragged with and it belongs to the pane beside this, which
//! is where the bars and the zones a row can be dropped on are: what the tree
//! does is hand the press over at the moment it begins, and draw the row it is
//! holding as lifted while it is being carried. So a press that let go about
//! where it landed is the press above, unchanged, and one that carried the row
//! somewhere opens the file there instead — see `Code.tsx`.
//!
//! Only a file. A folder row hands nothing over, there being nothing for a drop
//! to open, so a press on one is the expand it has always been. And nothing
//! about a drag writes anything: the row is still where it is and the folder
//! still open or closed, whatever the drag came to.
//!
//! **And a row drops a menu, which is where a file or a folder is made.** The
//! one `ContextMenu` the app has, with which row it is about held here rather
//! than by a menu per row — which is exactly how the tab bar beside the tree
//! opens its own.
//!
//! **Opened by a right-click, and by a long press on a folder row.** A phone
//! has no right-click and fires the same `contextmenu` from a long press, so
//! which hands reach one of these is the caller's answer — and the answer here
//! is different for the two kinds of row. A long press on a **file** row is
//! already how it is picked up and carried into a group, and that gesture is
//! spoken for, the way a card in the sidebar's is; nothing is picked up off a
//! **folder** row, so its long press is free and drops the menu. Which leaves a
//! touch screen everything a folder row offers — both makings, and the rename
//! and delete of the folder itself — and only a file row's own two out of
//! reach.
//!
//! **New file and New folder, on a folder row and on a root**, and on neither in
//! a read-only root — the root's own flag, which the roots listing already
//! carries, so the rows are not drawn rather than drawn to be refused.
//!
//! **And Rename and Delete, on every row but a root.** A root is a worktree
//! rather than anything in one, and what the human knows it by is the repository
//! it is a checkout of — so neither row is drawn there, which is also why it is
//! the one thing a row has to know about itself: the folder it was drawn in. A
//! row with none is a root. A file row offers those two and nothing else, there
//! being nowhere in a file to make anything.
//!
//! **And the name is typed in place.** For a making, the row pressed expands and
//! a new row appears under it carrying nothing but a field; for a rename, the
//! field is drawn *over* the row it is about, holding the name it has now. One
//! field either way, and one at a time: what is typed there is the name, Enter
//! does it, Escape leaves nothing behind, and a refusal is the server's sentence
//! drawn beside the field with what was typed still in it. A modal was decided
//! against — what is being named is a row in the tree, and the tree is where it
//! is seen.
//!
//! Afterwards the folder is read again, there being no watcher until stage 04,
//! and a new file opens as a tab in the active group the way a file pressed in
//! the tree does. A new folder opens nothing: there is nothing in it to open.
//!
//! **And a rename is handed to the pane beside the tree**, which is where every
//! other place the path is written down is: the tabs, the buffers, the bar a
//! refused save left standing, and the text this device is holding for a file it
//! has not saved. What the tree hands over is the two paths — see `Code.tsx`,
//! where a tab is retitled and re-keyed onto the new one. The folders open
//! beneath a renamed folder move with it, because they are kept there too.
//!
//! **And Delete asks first, in a card.** It is the one row here that cannot be
//! taken back, so what goes up is the confirm the app puts in front of anything
//! that cannot be — the same card a busy terminal and a dirty tab raise —
//! naming the row and, where it is a folder, saying that everything in it goes
//! with it. One confirm for a folder rather than one per file, the server taking
//! the whole of it in one call. A refusal comes back into that card, which is
//! where the press was made.
//!
//! **And a deleted file's tab is the pane's to keep**, the way a renamed one's
//! is: what the tree hands over is the path, and what happens to it is in
//! `Code.tsx` — the tab stays, read-only, saying the file is gone, with its text
//! still there to be copied out. A tab that vanished under somebody with unsaved
//! text would take the text with it.
//!
//! **And quick open is not in here**: the palette Ctrl+P drops is over the
//! pane rather than in the tree — see [`./Quick`] — because what it matches is
//! every root at once and what it opens is a tab. This half of the pane is how
//! a file is found by where it sits, and that one is how it is found by what it
//! is called.
//!
//! No git status marks yet, which are stage 04's with the watcher that keeps
//! them honest.

import {
  faChevronDown,
  faChevronRight,
} from "@fortawesome/free-solid-svg-icons";
import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  createUniqueId,
  type Accessor,
  type JSX,
  type Setter,
} from "solid-js";

import { Icon } from "../Icon";
import { ContextMenu } from "../Menu";
import { Modal } from "../Modal";
import {
  deletePath,
  listFileRoots,
  listFolder,
  makeFile,
  makeFolder,
  renamePath,
} from "../api/client";
import type {
  FileDeleted,
  FileMade,
  FileRenamed,
  FileRoot,
  FolderEntry,
  FolderListing,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import styles from "./Tree.module.css";

/// Each way a folder can come back holding nothing, in the words of what it is.
///
/// One sentence each rather than a single "could not be read", because the
/// server names them separately for exactly this: a path the pane should never
/// have asked for, a repository's insides, a worktree that has gone and a folder
/// that has are four different things, and only the human can tell which of them
/// they are looking at.
export const FOLDER_REFUSAL: Record<Extract<FolderListing, string>, string> = {
  Outside: "That is not in any of this conversation's worktrees.",
  UnderGit: "Code does not show what is inside a repository's .git.",
  RootGone: "This worktree is no longer on disk.",
  Missing: "That folder is no longer there.",
  NotAFolder: "That is not a folder.",
};

/// And each way a making can be refused, in the words of what it is.
///
/// The folder's sentences said about a row that is not there yet, with the one
/// that is a making's alone: a name already in the folder. Drawn beside the field
/// the name was typed into rather than where a folder's would go — what it is
/// about is what was just typed, and the field keeps it so the next Enter is a
/// correction rather than a retype.
export const MADE_REFUSAL: Record<Extract<FileMade, string>, string> = {
  Taken: "There is already something called that in this folder.",
  ReadOnly: "Nothing can be written in this worktree.",
  Outside: "That is not in any of this conversation's worktrees.",
  UnderGit: "Code does not touch what is inside a repository's .git.",
  RootGone: "This worktree is no longer on disk.",
  Missing: "That folder is no longer there.",
  NotAFolder: "That is not a folder.",
};

/// What a making came back saying, where it was refused — and `null` where it
/// landed.
///
/// The unwritable one is the server's own sentence, being the only one of them
/// that says something this side could not have worked out: permissions, a full
/// disk, or a folder that went between the check and the making.
export function madeRefusal(made: FileMade): string | null {
  if (typeof made !== "string") {
    return "Unwritable" in made ? made.Unwritable.why : null;
  }

  return MADE_REFUSAL[made];
}

/// And each way a rename can be refused, in the words of what it is.
///
/// The making's sentences said about something that is already there, with the
/// one that is a rename's alone: a root, which is a worktree rather than
/// anything in one. Drawn beside the field the name was typed into, where the
/// making's are, and for the reason they are — the field keeps what was typed so
/// the next Enter is a correction.
///
/// The row is never offered on a root, so that sentence is the endpoint's own
/// answer arriving where the tree cannot ask for it: said anyway, because a
/// refusal nobody worded is a refusal drawn as blank.
export const RENAMED_REFUSAL: Record<Extract<FileRenamed, string>, string> = {
  Taken: "There is already something called that in this folder.",
  IsRoot: "A worktree is not renamed from here.",
  ReadOnly: "Nothing can be written in this worktree.",
  Outside: "That is a path rather than a name.",
  UnderGit: "Code does not touch what is inside a repository's .git.",
  RootGone: "This worktree is no longer on disk.",
  Missing: "That is no longer there.",
};

/// What a rename came back saying, where it was refused — and `null` where it
/// landed.
///
/// The unwritable one is the server's own sentence, being the only one of them
/// that says something this side could not have worked out: permissions, or a
/// path that went between the check and the move.
export function renamedRefusal(renamed: FileRenamed): string | null {
  if (typeof renamed !== "string") {
    return "Unwritable" in renamed ? renamed.Unwritable.why : null;
  }

  return RENAMED_REFUSAL[renamed];
}

/// And each way a delete can be refused, in the words of what it is.
///
/// The rename's sentences less the one about a name: nothing is named here, so
/// nothing can be taken and nothing typed can be a path. Drawn in the card the
/// press was made in rather than beside a field — there is no field, and the
/// card is what is still on the screen when the answer comes back.
///
/// The root's is the endpoint's own answer arriving where the tree cannot ask
/// for it, the row never being offered on a root: worded anyway, because a
/// refusal nobody worded is a refusal drawn as blank.
export const DELETED_REFUSAL: Record<
  Exclude<Extract<FileDeleted, string>, "Deleted">,
  string
> = {
  IsRoot: "A worktree is not deleted from here.",
  ReadOnly: "Nothing can be written in this worktree.",
  Outside: "That is not in any of this conversation's worktrees.",
  UnderGit: "Code does not touch what is inside a repository's .git.",
  RootGone: "This worktree is no longer on disk.",
  Missing: "That is no longer there.",
};

/// What a delete came back saying, where it was refused — and `null` where it
/// landed.
///
/// The unwritable one is the server's own sentence, being the only one of them
/// that says something this side could not have worked out: permissions, a file
/// held open, or a folder that grew one between the walk and the removal.
export function deletedRefusal(deleted: FileDeleted): string | null {
  if (typeof deleted !== "string") {
    return "Unwritable" in deleted ? deleted.Unwritable.why : null;
  }

  return deleted === "Deleted" ? null : DELETED_REFUSAL[deleted];
}

/// And what the tree says when the conversation has no worktrees at all.
///
/// Which is a conversation before its grilling has cut one, and one that has
/// been closed and had them taken back. Neither is anything to report: there is
/// no tree to draw, and saying why is the whole of what there is to say.
export const NO_ROOTS =
  "This conversation has no worktree yet, so there are no files to show.";

/// What a listing came back saying, where it is saying something rather than
/// holding rows.
///
/// The unreadable one is the server's own sentence, being the only one of them
/// that says something this side could not have worked out — permissions, or a
/// folder that went between the ask and the reading.
export function refusal(listing: FolderListing): string | null {
  if (typeof listing !== "string") {
    return "Unreadable" in listing ? listing.Unreadable.why : null;
  }

  return FOLDER_REFUSAL[listing];
}

/// The rows a listing holds, or `null` where it is a sentence instead.
function rows(listing: FolderListing): FolderEntry[] | null {
  return typeof listing === "string" || !("Listed" in listing)
    ? null
    : listing.Listed.entries;
}

/// A new row being named, under the folder it is going into.
///
/// Which of the two kinds it will be is here because it is the whole of what
/// the field says about itself and what the request is: a file opens as a tab
/// afterwards, and a folder opens nothing.
interface Making {
  at: string;
  folder: boolean;
}

/// And a row of the tree being renamed, with the field drawn over it.
///
/// The folder it stands in as well as the path, because both are wanted: the
/// name it has now is the one cut off the other, and the folder is what is read
/// again once it has moved.
interface Renaming {
  over: string;
  within: string;
}

/// And a row being asked about before it is taken away.
///
/// [`Renaming`]'s two fields — the path, and the folder that is read again once
/// it has gone — with the one thing a card has to say that a path does not: a
/// folder takes its contents with it, and a file does not.
///
/// Not a [`Field`], because it is not one: what a delete opens is a card over
/// the page rather than a field on a row, and both may be open at once.
interface Removal extends Renaming {
  folder: boolean;
}

/// The one field the tree ever has open, and what it is about.
///
/// One at a time, because there is one field — see the signal in [`Tree`]. Two
/// shapes rather than one with a kind beside it, for the reason a tab is two:
/// they are drawn in different places and nothing ever asks which it is holding
/// without then using the answer.
type Field = Making | Renaming;

/// Which row of the tree a field is about, where it is about one that is drawn.
///
/// The folder a new row is going into, or the row being renamed — which is the
/// one thing both shapes have to answer, so that a folder shut while a field is
/// open anywhere inside it takes the field with it.
function about(field: Field | null): string | undefined {
  if (field === null) {
    return undefined;
  }

  return "over" in field ? field.over : field.at;
}

/// And what a row inside a folder is called: the path with the folder cut off
/// the front of it.
///
/// Cut rather than split on a separator, because a worktree on Windows is
/// spelled with a `\` and a path here is spelled the way the root it came from
/// is: what the server built was this folder joined to this name, so what is
/// left after the folder and the one separator after it is the name.
function named(path: string, within: string): string {
  return path.slice(within.length + 1);
}

export function Tree(props: {
  conversation: number;
  /// What a file pressed in it opens: the path, handed to the groups of tabs
  /// beside the tree, where it opens in the active one.
  open: (path: string) => void;

  /// And a press on a file row beginning, which the pane beside the tree takes
  /// over: it may turn out to be a drag, and where such a drag is let go is a
  /// group of that pane rather than anything here.
  ///
  /// Handed the event rather than the path alone, because what the pane does
  /// with it is the pointer's own gesture — the capture, the grace, the hold
  /// under a finger — which is the gesture a tab is dragged with.
  pick: (event: PointerEvent, path: string) => void;

  /// And which file it is carrying, where it is carrying one: the row is drawn
  /// lifted from the moment it leaves until the hand lets go, the way the tab
  /// in a hand is. Null every moment nobody is dragging one.
  carried: Accessor<string | null>;

  /// And what a rename in it came to: where the row was, and where it now is.
  ///
  /// Handed over because every other place a path is written down is the pane's
  /// — the tabs, the buffers behind them, the bar a refused save left standing,
  /// and the text this device is holding for a file nobody has saved. A tab
  /// follows its file, and this is the whole of what the tree has to say about
  /// it (ADR 0019, *The tree*).
  ///
  /// The folders open under a renamed folder move with it there too, the tree's
  /// own being kept beside the tabs — see `keeping.ts`.
  renamed: (from: string, to: string) => void;

  /// And what a delete in it came to: the path that is no longer there.
  ///
  /// Handed over for the rename's reason, and for one of its own: a tab whose
  /// file has gone **stays**, read-only, saying so, with its text still there to
  /// be copied out — a tab that vanished under somebody with unsaved text would
  /// take the text with it (ADR 0019, *The tree*). Which is the pane's to do,
  /// every place a path is written down being over there.
  ///
  /// A folder carries everything under it here too, the one press having taken
  /// the whole of it.
  deleted: (path: string) => void;

  /// Which folders are open, and what each of them last read.
  ///
  /// A path is in here or it is not, and that is the whole of what open means:
  /// collapsing one takes its entry away, so expanding it again is a fresh
  /// reading rather than a look at what was drawn before.
  ///
  /// Held above the pane rather than here, with the tabs and the buffers and
  /// for their reason: the details pane is taken down whenever something else
  /// is drawn in it, and a tree that came back to its roots every time an
  /// Event was opened would be the walk down to a file made again — see
  /// `keeping.ts`.
  held: Accessor<Record<string, FolderListing>>;
  setHeld: Setter<Record<string, FolderListing>>;
}): JSX.Element {
  /// The worktrees this conversation has.
  ///
  /// Merged by path rather than frozen: a companion added to a drafting
  /// conversation, or a worktree cut as grilling starts, is a root that was not
  /// there a moment ago — and a Nudge is what says the record moved. What is
  /// *inside* a root is not in this reading at all, so a re-read costs a row
  /// apiece and disturbs nothing that is open.
  const roots = useReading(() => ({
    queryKey: ["file-roots", props.conversation],
    queryFn: () => listFileRoots(props.conversation),
    freshness: { reconcile: "path" },
  }));

  /// Which folders are open, and what each of them last read — see the prop,
  /// which is where it is kept and why it is not kept here.
  const held = props.held;
  const setHeld = props.setHeld;

  /// And which are open with nothing read back yet, which is the moment between
  /// the press and the answer.
  ///
  /// This one *is* the tree's own, unlike the listings above: a read in flight
  /// belongs to the mount that made it, and a pane taken down while one was
  /// out has no caret left to spin. What lands afterwards lands in the keeping
  /// all the same, so the folder is open when the pane comes back.
  const [reading, setReading] = createSignal<string[]>([]);

  /// Whether a folder is open, which is the caret and the rows under it.
  ///
  /// Named apart from the press that opens a *file*: one is a fact about a row
  /// of this tree, and the other is what the pane beside it does with a path.
  const expanded = (path: string): boolean =>
    held()[path] !== undefined || reading().includes(path);

  /// Which row a menu is open over, where one is: where the pointer was, the
  /// row's path, whether it is a folder, and the folder it was drawn in.
  ///
  /// The whole of what the menu is about, and `null` while nothing is open. Held
  /// here rather than by the row, which is what one `ContextMenu` for the tree
  /// means — the tab bar beside it holds its own the same way.
  ///
  /// **`within` is what says a row is not a root**: every row of the tree is
  /// drawn inside a folder except the roots, which are drawn inside nothing. So
  /// it is what the Rename row is offered on, and it is where the name typed in
  /// is joined back on.
  const [pointed, setPointed] = createSignal<{
    at: { x: number; y: number };
    path: string;
    folder: boolean;
    within?: string;
  } | null>(null);

  /// Whether the gesture that is opening a menu began under a finger.
  ///
  /// A phone has no right-click and fires `contextmenu` from a long press, and
  /// on a **file** row that press is already spoken for: it is how the row is
  /// picked up and dragged into a group. So a file row's menu is the mouse's
  /// alone — and a **folder** row's is not, nothing being picked up off one, so
  /// a long press there drops it the way a right-click does.
  ///
  /// What tells the two hands apart is the pointer that began the press rather
  /// than the event itself, which carries nothing about the hand that made it —
  /// the sidebar's cards and the tab bar say the same thing the same way.
  let fromTouch = false;

  /// The one field the tree has open, where it has one.
  ///
  /// One at a time and one signal for both, because there is one field: a name
  /// for a row that is not there yet, drawn under the folder it is going into;
  /// or the name of a row that is, drawn over the row itself. What is typed and
  /// what the last Enter was refused with belong to whichever it is.
  const [field, setField] = createSignal<Field | null>(null);

  /// What has been typed into that field so far.
  ///
  /// Held here rather than in the field, so that a refusal keeps it: what comes
  /// back is a sentence to read beside what was typed, and a field that emptied
  /// itself would make a correction a retype.
  const [typed, setTyped] = createSignal("");

  /// And what the server refused the last Enter with, where it refused one.
  const [said, setSaid] = createSignal<string | null>(null);

  /// Which row a Delete has been pressed on and is being asked about, where one
  /// is: the path, whether it is a folder, and the folder it stands in.
  ///
  /// The whole of what the card says, and `null` while no card is up. Its own
  /// signal beside the field above rather than a third shape of it: a card over
  /// the page and a field on a row are not one thing drawn two ways, and both
  /// may be open at once — a name half typed on one row is not a reason to
  /// refuse a press on another.
  const [removing, setRemoving] = createSignal<Removal | null>(null);

  /// And what the server refused that press with, where it refused one — drawn
  /// in the card, which is where the press was made.
  const [refused, setRefused] = createSignal<string | null>(null);

  /// Whether an Enter is in flight, so that a second one while the first is
  /// still being answered does not make two rows or move one twice.
  let working = false;

  /// Which root a path is in, or nothing where it is under none of them.
  ///
  /// Read off the roots listing rather than carried down the rows, because what
  /// it is wanted for is one press: whether the menu has anything to offer over
  /// this row, which is the root's own writable flag.
  const rooted = (path: string): FileRoot | undefined =>
    roots.data?.roots.find((root) => path.startsWith(root.path));

  /// Read a folder off the disk and hold what came back.
  ///
  /// Every time, because nothing here is told when the disk moves and the folder
  /// may have been rewritten by the agent since it was last drawn — which is why
  /// a making reads it again rather than adding the row it just asked for, and
  /// why a rename does rather than moving one.
  const read = (path: string): Promise<void> => {
    setReading((was) => (was.includes(path) ? was : [...was, path]));

    return (
      listFolder(props.conversation, path)
        .then((listing) => {
          setHeld((was) => ({ ...was, [path]: listing }));
        })
        // A request that never landed is a folder that says why it is empty, the
        // way a folder the server refused does: the sentence is the server's
        // where there is one, and this is the sentence there is instead.
        .catch((error: Error) => {
          setHeld((was) => ({
            ...was,
            [path]: { Unreadable: { why: error.message } },
          }));
        })
        .finally(() => setReading((was) => was.filter((one) => one !== path)))
    );
  };

  /// Open a folder, or shut it.
  ///
  /// Shutting forgets what it held, and takes the field with it where the field
  /// is anywhere inside: a row being named is a row of that folder, and a folder
  /// that is shut is not drawing its rows.
  const toggle = (path: string): void => {
    if (expanded(path)) {
      if (about(field())?.startsWith(path) === true) leave();

      setReading((was) => was.filter((one) => one !== path));
      setHeld((was) => {
        const rest = { ...was };
        delete rest[path];
        return rest;
      });
      return;
    }

    void read(path);
  };

  /// A right-click on a row asks what can be done with it.
  ///
  /// The browser's own menu is not what the hand is asking for, so that goes —
  /// and only where there is a menu to put in its place: a row of a read-only
  /// root has nothing on its, and a card with no rows in it is worse than the
  /// browser's own. Every other row has something — a folder makes, and
  /// everything but a root renames.
  ///
  /// Both hands on a folder row and the mouse's alone on a file row, for the
  /// reason [`fromTouch`] is kept: a long press on a file row is how it is
  /// carried into a group, and there is nothing to carry off a folder.
  const ask = (
    event: MouseEvent,
    path: string,
    folder: boolean,
    within?: string,
  ): void => {
    if ((fromTouch && !folder) || !rooted(path)?.writable) {
      return;
    }

    event.preventDefault();
    setPointed({ at: { x: event.clientX, y: event.clientY }, path, folder, within });
  };

  /// Start naming one: the folder opens, and a row carrying nothing but a field
  /// appears under it.
  ///
  /// The folder opens because the field is drawn among its rows, and a field
  /// inside a folder nobody can see would be a press that did nothing. Already
  /// open, it stays as it is — what is in it was read when it was opened, and
  /// the making reads it again afterwards.
  const begin = (at: string, folder: boolean): void => {
    opening({ at, folder }, "");

    if (!expanded(at)) void read(at);
  };

  /// And start renaming one, which is the field drawn over the row it is about
  /// with the name it has now in it.
  ///
  /// Nothing opens and nothing is read: the row is already drawn, and the field
  /// stands where it stands.
  const rename = (over: string, within: string): void => {
    opening({ over, within }, named(over, within));
  };

  /// And ask about taking one away, which is the card rather than a field:
  /// Delete is the one row of this menu that cannot be taken back, so what the
  /// press opens is the confirm the app puts in front of anything that cannot be
  /// (ADR 0019, *The tree*).
  ///
  /// The field is left exactly where it is. A name half typed on one row is not
  /// something a press on another should throw away, and the card is drawn over
  /// the page rather than among the rows.
  const asking = (over: string, within: string, folder: boolean): void => {
    setPointed(null);
    setRefused(null);
    setRemoving({ over, within, folder });
  };

  /// Either of them: the menu goes, and the field opens holding whatever it
  /// starts with. Named apart from `props.open`, which is what a file pressed
  /// in the tree does.
  const opening = (what: Field, holding: string): void => {
    setPointed(null);
    setTyped(holding);
    setSaid(null);
    setField(what);
  };

  /// And leave off, which takes the field away and leaves nothing behind.
  const leave = (): void => {
    setField(null);
    setTyped("");
    setSaid(null);
  };

  /// Do whatever the field is open for, under the name that has been typed.
  ///
  /// Nothing at all typed is not a name, so Enter over an empty field is a press
  /// that waits rather than one that asks: there is nothing for the server to
  /// refuse that the field has not already said by being empty.
  const commit = (): void => {
    const asked = field();
    const name = typed().trim();

    if (asked === null || name === "" || working) {
      return;
    }

    if ("over" in asked) {
      move(asked, name);
      return;
    }

    make(asked, name);
  };

  /// Make it, under the name that has been typed.
  ///
  /// Afterwards the folder is read again — there being no watcher until stage 04
  /// — and a new file opens as a tab in the active group the way a file pressed
  /// in the tree does. A new folder opens nothing.
  const make = (asked: Making, name: string): void => {
    working = true;
    setSaid(null);

    const at = `${asked.at}/${name}`;
    const asking = asked.folder
      ? makeFolder(props.conversation, at)
      : makeFile(props.conversation, at);

    void asking
      .then((made) => {
        if (typeof made === "string" || !("Made" in made)) {
          setSaid(madeRefusal(made));
          return;
        }

        leave();

        void read(asked.at);

        if (!asked.folder) props.open(made.Made.path);
      })
      // A request that never landed is the same thing to say as one the server
      // refused: a sentence beside the field, with what was typed still in it.
      .catch((error: Error) => setSaid(error.message))
      .finally(() => {
        working = false;
      });
  };

  /// And move it, which is the rename.
  ///
  /// **A name rather than a path**, which is what keeps it inside the root it is
  /// in: the field holds one segment and the server joins it back onto the folder
  /// the row is already in, so there is no shape a request to cross two roots
  /// could have (ADR 0019, *The tree*).
  ///
  /// The name it already has is nothing to ask for, so Enter over an untouched
  /// field is the press that waits an empty one is: the server would answer that
  /// the name is taken, which is true and is not what happened.
  ///
  /// Afterwards the folder is read again, as it is for a file made — and the two
  /// paths go to the pane beside the tree, where every tab of that file follows
  /// it.
  const move = (asked: Renaming, name: string): void => {
    if (name === named(asked.over, asked.within)) {
      leave();
      return;
    }

    working = true;
    setSaid(null);

    void renamePath(props.conversation, asked.over, name)
      .then((done) => {
        if (typeof done === "string" || !("Renamed" in done)) {
          setSaid(renamedRefusal(done));
          return;
        }

        leave();

        props.renamed(asked.over, done.Renamed.path);

        void read(asked.within);
      })
      .catch((error: Error) => setSaid(error.message))
      .finally(() => {
        working = false;
      });
  };

  /// And take it away, which is the card's own press made.
  ///
  /// **One call for a folder**, everything under it going with it: one confirm
  /// in front of the press is enough because there is one press behind it.
  ///
  /// Afterwards the folder above the row is read again, as it is for a making
  /// and a rename — and the path goes to the pane beside the tree, where the tab
  /// of a file that has gone stays read-only with its text still in it.
  const remove = (asked: Removal): void => {
    working = true;
    setRefused(null);

    void deletePath(props.conversation, asked.over)
      .then((done) => {
        if (done !== "Deleted") {
          setRefused(deletedRefusal(done));
          return;
        }

        setRemoving(null);

        // A field anywhere inside what has just gone is a field on a row that is
        // not there any more — a folder shut takes one the same way.
        if (about(field())?.startsWith(asked.over) === true) leave();

        props.deleted(asked.over);

        void read(asked.within);
      })
      // A request that never landed is the same thing to say as one the server
      // refused: a sentence in the card, with the card still up.
      .catch((error: Error) => setRefused(error.message))
      .finally(() => {
        working = false;
      });
  };

  return (
    <div class={styles.tree} aria-label="This conversation's files">
      <Switch fallback={<Empty>Reading this conversation's worktrees…</Empty>}>
        <Match when={roots.isError}>
          <ErrorLine>
            Could not read this conversation's worktrees: {roots.error?.message}
          </ErrorLine>
        </Match>
        <Match when={roots.data && roots.data.roots.length > 0}>
          <ul class={styles.rows}>
            <For each={roots.data?.roots}>
              {(root) => (
                <Row
                  name={<RootName root={root} />}
                  path={root.path}
                  folder={true}
                  depth={0}
                  expanded={expanded}
                  held={held}
                  toggle={toggle}
                  open={props.open}
                  pick={props.pick}
                  carried={props.carried}
                  ask={ask}
                  touched={(touch) => {
                    fromTouch = touch;
                  }}
                  field={field}
                  typed={typed}
                  setTyped={setTyped}
                  said={said}
                  commit={commit}
                  leave={leave}
                />
              )}
            </For>
          </ul>
        </Match>
        <Match when={roots.data}>
          <Empty>{NO_ROOTS}</Empty>
        </Match>
      </Switch>

      {/* And what a right-click on a row drops: the two things that can be made
          in a folder, and the rename every row but a root offers. The tree's
          only menu, and the same component the tab bar beside it opens its own
          with (ADR 0019, *The tree*).

          Outside the Switch above because it is drawn over the page rather than
          among the rows: what is behind it is whichever of those arms the tree is
          in. */}
      <ContextMenu
        class={styles.rowActions!}
        name="Row actions"
        at={pointed()?.at ?? null}
        close={() => setPointed(null)}
      >
        {() => (
          <>
            {/* A folder makes, a root among them — there is nowhere for a file
                row to make anything. */}
            <Show when={pointed()?.folder}>
              <For
                each={
                  [
                    ["New file", false],
                    ["New folder", true],
                  ] as const
                }
              >
                {([says, folder]) => (
                  <button
                    type="button"
                    role="menuitem"
                    class={folder ? styles.newFolder : styles.newFile}
                    onClick={() => {
                      const asked = pointed();

                      if (asked !== null) begin(asked.path, folder);
                    }}
                  >
                    {says}
                  </button>
                )}
              </For>
            </Show>

            {/* And everything but a root renames and deletes: a root is a
                worktree rather than anything in one, and a row with no folder
                around it is a root. In the order ADR 0019 names the four in,
                which is also the order of what they cost. */}
            <Show when={pointed()?.within}>
              {(within) => (
                <>
                  <button
                    type="button"
                    role="menuitem"
                    class={styles.rename}
                    onClick={() => {
                      const asked = pointed();

                      if (asked !== null) rename(asked.path, within());
                    }}
                  >
                    Rename
                  </button>

                  <button
                    type="button"
                    role="menuitem"
                    class={styles.delete}
                    onClick={() => {
                      const asked = pointed();

                      if (asked !== null) {
                        asking(asked.path, within(), asked.folder);
                      }
                    }}
                  >
                    Delete
                  </button>
                </>
              )}
            </Show>
          </>
        )}
      </ContextMenu>

      {/* And the card that Delete puts up before anything goes, which the press
          that made it is waiting on. Outside the Switch above for the menu's
          reason: it is drawn over the page rather than among the rows. */}
      <Removing
        asked={removing()}
        said={refused}
        keep={() => setRemoving(null)}
        remove={() => {
          const asked = removing();

          if (asked !== null && !working) remove(asked);
        }}
      />
    </div>
  );
}

/// What a Delete on a row is answered with, before anything is taken away:
/// which row it is, what is about to happen to it, and the two ways out.
///
/// **The app's own confirm card**, the one a busy terminal and a tab with
/// unsaved text in it raise — the same modal, the same pair of presses, the same
/// shape — because it is the same question: a press that cannot be taken back,
/// put back to the human before it is made (ADR 0019, *The tree*).
///
/// **A folder says it takes its contents with it**, which is the one thing this
/// card says that the others do not have to: what is about to go is a row of a
/// tree, and how much is under that row is exactly what the human cannot see
/// from the row itself. One confirm for the whole of it rather than one per
/// file.
///
/// And a refusal is drawn in here rather than beside the row: the card is what
/// is on the screen when the answer comes back, and what it is about is the
/// press that was made in it.
function Removing(props: {
  /// The row being asked about, or `null` while nothing is.
  asked: Removal | null;
  /// What the server refused the press with, where it refused one.
  said: Accessor<string | null>;
  /// The way back, which Escape and a press on the backdrop come to as well:
  /// every way out of this card but the one button keeps the row.
  keep: () => void;
  /// And the press it asked about, made.
  remove: () => void;
}): JSX.Element {
  // Generated rather than written, the way every other card in the app names
  // itself: more than one pane stands on a page at once.
  const id = createUniqueId();

  /// What the row is called, which is what the card names throughout: the path
  /// with the folder it stands in cut off the front of it.
  const name = (): string =>
    props.asked === null ? "" : named(props.asked.over, props.asked.within);

  return (
    <Modal
      class={styles.confirming!}
      open={props.asked !== null}
      close={props.keep}
      labelledBy={id}
    >
      <p id={id} class={styles.confirmingTitle}>
        Delete this {props.asked?.folder === true ? "folder" : "file"}?
      </p>
      <p class={styles.confirmingWhy}>
        <Show
          when={props.asked?.folder === true}
          fallback={`${name()} will be taken off the disk.`}
        >
          {`${name()} will be taken off the disk, and everything in it goes with it.`}
        </Show>{" "}
        This cannot be undone.
      </p>

      {/* And what the server refused it with, where it refused one — in the
          card, with the card still up. */}
      <Show when={props.said()}>
        {(why) => <ErrorLine class={styles.refused}>{why()}</ErrorLine>}
      </Show>

      <div class={styles.confirmingOut}>
        {/* Both classes, as every other confirm pair in the app carries them:
            the global one is the paint, and the module's is what the row above
            stands the filled press out of. */}
        <button
          type="button"
          class={`${styles.secondary!} secondary`}
          onClick={() => props.keep()}
        >
          Keep it
        </button>
        <button type="button" onClick={() => props.remove()}>
          Delete {name()}
        </button>
      </div>
    </Modal>
  );
}

/// What a root is called: the repository it is a checkout of, with a word about
/// a companion nothing can be written in.
///
/// The mark is on the row rather than in a legend, because it is the one thing
/// about a root that changes what a press inside it will do — a file opened out
/// of one takes no typing, and a human who finds that out by typing has been
/// told too late.
function RootName(props: { root: FileRoot }): JSX.Element {
  return (
    <>
      <span class={styles.repo}>{props.root.repo}</span>
      <Show when={!props.root.writable}>
        <span class={styles.readOnly}>read-only</span>
      </Show>
    </>
  );
}

/// One row of the tree, and — where it is an open folder — what is under it.
///
/// Both kinds are buttons, because pressing either does something: a folder is
/// read off the disk, and a file is opened as a tab in the active group beside
/// the tree. What tells them apart in the markup is the caret and `aria-expanded`,
/// which a file has neither of.
///
/// **Except while it is being renamed**, when the field stands where the button
/// does: what is being named is this row, and the tree is where it is seen. A
/// folder goes on drawing what is under it while its own name is typed over —
/// the rows below are still there, and a field is not a reason to hide them.
function Row(props: {
  /// What the row says, which is a repository's name for a root and an entry's
  /// own name for everything below one.
  name: JSX.Element;
  path: string;
  folder: boolean;
  /// The folder this row is drawn in, or nothing where it is a root.
  ///
  /// Which is the whole of what says a row is a root: every other row of the
  /// tree came out of a listing of some folder. It is what the Rename row is
  /// offered on, and what the name typed into the field is joined back onto.
  within?: string;
  /// How deep it is, which is the whole of what indents it.
  depth: number;
  /// Whether this folder is open, which a file is never.
  expanded: (path: string) => boolean;
  held: () => Record<string, FolderListing>;
  /// Open this folder or shut it.
  toggle: (path: string) => void;
  /// And open this file, which is the groups of tabs beside the tree.
  open: (path: string) => void;
  /// And pick it up, which is the same pane taking the press over.
  pick: (event: PointerEvent, path: string) => void;
  /// And which file that pane is carrying, where it is carrying one.
  carried: Accessor<string | null>;
  /// And drop the menu over it, which the tree holds the one of.
  ask: (
    event: MouseEvent,
    path: string,
    folder: boolean,
    within?: string,
  ) => void;
  /// Said as any press on any row begins, with which hand made it: the menu is
  /// the mouse's alone, and `contextmenu` is the one event that cannot say.
  touched: (touch: boolean) => void;
  /// The one field the tree has open, where it has one — and so whether it is
  /// drawn among *this* folder's rows, or over this row itself.
  field: Accessor<Field | null>;
  /// What has been typed into it so far, and the typing itself.
  typed: Accessor<string>;
  setTyped: Setter<string>;
  /// And what the last Enter was refused with, where it was refused.
  said: Accessor<string | null>;
  /// Enter: make it or move it, under the name that has been typed.
  commit: () => void;
  /// And Escape: take the field away and leave nothing behind.
  leave: () => void;
}): JSX.Element {
  /// The indent, in the one unit a tree has: a level.
  const inset = (): string => `${0.5 + props.depth * 0.75}rem`;

  /// And a file's, which stands where a folder's caret would be: the caret's own
  /// width and the gap beside it, so the names line up down the column whichever
  /// kind each row is. Said here rather than in the sheet because the indent
  /// itself is a style on the row, and one rule cannot be half of two.
  const fileInset = (): string => `calc(${inset()} + 0.7em + 0.4rem)`;

  const listing = (): FolderListing | undefined => props.held()[props.path];

  /// Whether a new row is being named in this folder, which is the field drawn
  /// among its rows.
  const making = (): Making | null => {
    const has = props.field();

    return has !== null && "at" in has && has.at === props.path ? has : null;
  };

  /// And whether this row is the one being renamed, which is the field drawn
  /// over it.
  const renaming = (): boolean => {
    const has = props.field();

    return has !== null && "over" in has && has.over === props.path;
  };

  return (
    <li class={styles.row}>
      <Show
        when={props.folder}
        fallback={
          <Show
            when={!renaming()}
            fallback={
              <Naming
                called="New name"
                inset={fileInset()}
                typed={props.typed}
                setTyped={props.setTyped}
                said={props.said}
                commit={props.commit}
                leave={props.leave}
              />
            }
          >
            <button
              type="button"
              class={styles.file}
              classList={{ [styles.lifted!]: props.carried() === props.path }}
              style={{ "padding-left": fileInset() }}
              // Which of three things this press is — one that opens the file,
              // one that scrolls the tree, or a drag — is settled by what the
              // hand does next, and by the pane beside the tree: the bars and
              // the zones such a drag lands on are all over there.
              onPointerDown={(event) => {
                props.touched(event.pointerType !== "mouse");
                props.pick(event, props.path);
              }}
              onClick={() => props.open(props.path)}
              onContextMenu={(event) =>
                props.ask(event, props.path, false, props.within)
              }
            >
              <span class={styles.name}>{props.name}</span>
            </button>
          </Show>
        }
      >
        <Show
          when={!renaming()}
          fallback={
            <Naming
              called="New name"
              inset={inset()}
              typed={props.typed}
              setTyped={props.setTyped}
              said={props.said}
              commit={props.commit}
              leave={props.leave}
            />
          }
        >
          <button
            type="button"
            class={styles.folder}
            style={{ "padding-left": inset() }}
            aria-expanded={props.expanded(props.path)}
            // Nothing is picked up off a folder row — there is nothing for a
            // drop to open — so this says only which hand is pressing, which is
            // what the menu below needs and the whole of what a folder's press
            // gives the pointer.
            onPointerDown={(event) =>
              props.touched(event.pointerType !== "mouse")
            }
            onClick={() => props.toggle(props.path)}
            onContextMenu={(event) =>
              props.ask(event, props.path, true, props.within)
            }
          >
            <Icon
              of={props.expanded(props.path) ? faChevronDown : faChevronRight}
              class={styles.caret}
            />
            <span class={styles.name}>{props.name}</span>
          </button>
        </Show>

        {/* And the row a name is typed into, where this is the folder it is
            being made in: under the row pressed and above what the folder
            holds, which is where the row it becomes will be drawn. */}
        <Show when={making()}>
          {(asked) => (
            <ul class={styles.rows}>
              <li class={styles.row}>
                <Naming
                  called={asked().folder ? "New folder name" : "New file name"}
                  inset={`calc(${inset()} + 0.75rem)`}
                  typed={props.typed}
                  setTyped={props.setTyped}
                  said={props.said}
                  commit={props.commit}
                  leave={props.leave}
                />
              </li>
            </ul>
          )}
        </Show>

        <Show when={listing()}>
          {(read) => (
            <Show
              when={rows(read())}
              fallback={
                <p class={styles.why} style={{ "padding-left": inset() }}>
                  {refusal(read())}
                </p>
              }
            >
              {(entries) => (
                <ul class={styles.rows}>
                  <For each={entries()}>
                    {(entry) => (
                      <Row
                        name={entry.name}
                        path={entry.path}
                        folder={entry.folder}
                        within={props.path}
                        depth={props.depth + 1}
                        expanded={props.expanded}
                        held={props.held}
                        toggle={props.toggle}
                        open={props.open}
                        pick={props.pick}
                        carried={props.carried}
                        ask={props.ask}
                        touched={props.touched}
                        field={props.field}
                        typed={props.typed}
                        setTyped={props.setTyped}
                        said={props.said}
                        commit={props.commit}
                        leave={props.leave}
                      />
                    )}
                  </For>
                </ul>
              )}
            </Show>
          )}
        </Show>
      </Show>
    </li>
  );
}

/// The field a row is named in, and the sentence a refusal came back with.
///
/// **A field on a row rather than a modal** — what is being named is a row in the
/// tree, and the tree is where it is seen (ADR 0019, *The tree*). For a making it
/// is drawn where the row it becomes will be, indented one level in from the
/// folder it is going into; for a rename it stands where the row itself does,
/// holding the name it has now.
///
/// Enter does it and Escape takes it away, and nothing else does either: a blur
/// leaves the field standing, because the sentence a refusal draws is beside it
/// and a field that went when the eye moved would take the sentence with it.
///
/// The text is the tree's rather than this field's, so that a refusal keeps it —
/// see the signal in [`Tree`], where the reason is.
function Naming(props: {
  /// What the field says about itself: what is read aloud, and what a human sees
  /// as the placeholder. A rename's is never seen as a placeholder — the field
  /// opens with the name in it — and is still what a screen reader reads.
  called: string;
  /// How far in it stands, which is the indent of the row it is or will be.
  inset: string;
  typed: Accessor<string>;
  setTyped: Setter<string>;
  said: Accessor<string | null>;
  commit: () => void;
  leave: () => void;
}): JSX.Element {
  return (
    <div class={styles.naming} style={{ "padding-left": props.inset }}>
      <input
        class={styles.field}
        type="text"
        value={props.typed()}
        aria-label={props.called}
        placeholder={props.called}
        // The field is what the press asked for, so it takes the keyboard on the
        // way in: a row that appeared and then waited to be pressed would be two
        // presses for one gesture. And what is in it is taken with it, so that a
        // rename is typed over rather than backspaced out — a new row's field is
        // empty, and selecting nothing is nothing.
        ref={(field) =>
          queueMicrotask(() => {
            field.focus();
            field.select();
          })
        }
        onInput={(event) => props.setTyped(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            props.commit();
          }

          // Kept off the pane behind it: Escape over a tree is this field going,
          // rather than anything the page around it does with the key.
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            props.leave();
          }
        }}
      />

      {/* And what the server refused it with, under the field it is about, in
          the one colour the app means a refusal by. */}
      <Show when={props.said()}>
        {(why) => <ErrorLine class={styles.refused}>{why()}</ErrorLine>}
      </Show>
    </div>
  );
}
