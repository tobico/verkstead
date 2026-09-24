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
//! one `ContextMenu` the app has, opened by a right-click, with which row it is
//! about held here rather than by a menu per row — which is exactly how the tab
//! bar beside the tree opens its own. **The mouse's alone**: a long press on a
//! file row is how it is picked up and dragged into a group, so the gesture is
//! spoken for, and the sidebar's cards answer the same problem the same way.
//! Code is desktop-first and ADR 0019 promises nothing on a phone, so a touch
//! screen gets no row menu rather than a second gesture invented for one.
//!
//! **New file and New folder, on a folder row and on a root**, and on neither in
//! a read-only root — the root's own flag, which the roots listing already
//! carries, so the rows are not drawn rather than drawn to be refused. A file
//! row offers neither, so nothing comes down over one at all: the rows that will
//! be there for a file are rename and delete, which are the two tasks after
//! this.
//!
//! **And the name is typed in place.** The row pressed expands, a new row
//! appears under it carrying nothing but a field, and what is typed there is the
//! name. Enter makes it, Escape takes the row away and leaves nothing behind,
//! and a refusal is the server's sentence drawn beside the field with what was
//! typed still in it. A modal was decided against — what is being named is a row
//! in the tree, and the tree is where it is seen.
//!
//! Afterwards the folder is read again, there being no watcher until stage 04,
//! and a new file opens as a tab in the active group the way a file pressed in
//! the tree does. A new folder opens nothing: there is nothing in it to open.
//!
//! No rename, no delete and no quick open yet: those are the rest of this stage.
//! And no git status marks, which are stage 04's with the watcher that keeps them
//! honest.

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
  type Accessor,
  type JSX,
  type Setter,
} from "solid-js";

import { Icon } from "../Icon";
import { ContextMenu } from "../Menu";
import { listFileRoots, listFolder, makeFile, makeFolder } from "../api/client";
import type {
  FileMade,
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
  /// row's path, and whether it is a folder.
  ///
  /// The whole of what the menu is about, and `null` while nothing is open. Held
  /// here rather than by the row, which is what one `ContextMenu` for the tree
  /// means — the tab bar beside it holds its own the same way.
  const [pointed, setPointed] = createSignal<{
    at: { x: number; y: number };
    path: string;
  } | null>(null);

  /// Whether the gesture that is opening a menu began under a finger.
  ///
  /// A phone has no right-click and fires `contextmenu` from a long press, which
  /// is the gesture a file row is dragged into a group with. So the menu is the
  /// mouse's alone, and what tells the two apart is the pointer that started the
  /// press rather than the event itself, which carries nothing about the hand
  /// that made it — the sidebar's cards and the tab bar say the same thing the
  /// same way.
  let fromTouch = false;

  /// Which folder a name is being typed into, where one is: the folder the new
  /// row goes in, and which of the two kinds it will be.
  const [naming, setNaming] = createSignal<{
    at: string;
    folder: boolean;
  } | null>(null);

  /// What has been typed into that field so far.
  ///
  /// Held here rather than in the field, so that a refusal keeps it: what comes
  /// back is a sentence to read beside what was typed, and a field that emptied
  /// itself would make a correction a retype.
  const [typed, setTyped] = createSignal("");

  /// And what the server refused the last Enter with, where it refused one.
  const [said, setSaid] = createSignal<string | null>(null);

  /// Whether a making is in flight, so that a second Enter while the first is
  /// still being answered does not make two rows.
  let making = false;

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
  /// a making reads it again rather than adding the row it just asked for.
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
  /// Shutting forgets what it held, and takes any field open in it with it: a
  /// row being named is a row of that folder, and a folder that is shut is not
  /// drawing its rows.
  const toggle = (path: string): void => {
    if (expanded(path)) {
      if (naming()?.at === path) leave();

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
  /// and only where there is a menu to put in its place: a file row and a row of
  /// a read-only root have nothing on theirs yet, and a card with no rows in it
  /// is worse than the browser's own.
  ///
  /// A mouse's gesture and only a mouse's, for the reason [`fromTouch`] is kept.
  const ask = (event: MouseEvent, path: string, folder: boolean): void => {
    if (fromTouch || !folder || !rooted(path)?.writable) {
      return;
    }

    event.preventDefault();
    setPointed({ at: { x: event.clientX, y: event.clientY }, path });
  };

  /// Start naming one: the folder opens, and a row carrying nothing but a field
  /// appears under it.
  ///
  /// The folder opens because the field is drawn among its rows, and a field
  /// inside a folder nobody can see would be a press that did nothing. Already
  /// open, it stays as it is — what is in it was read when it was opened, and
  /// the making reads it again afterwards.
  const begin = (at: string, folder: boolean): void => {
    setPointed(null);
    setTyped("");
    setSaid(null);
    setNaming({ at, folder });

    if (!expanded(at)) void read(at);
  };

  /// And leave off naming, which takes the row away and leaves nothing behind.
  const leave = (): void => {
    setNaming(null);
    setTyped("");
    setSaid(null);
  };

  /// Make it, under the name that has been typed.
  ///
  /// Nothing at all typed is not a name, so Enter over an empty field is a press
  /// that waits rather than one that asks: there is nothing for the server to
  /// refuse that the field has not already said by being empty.
  ///
  /// Afterwards the folder is read again — there being no watcher until stage 04
  /// — and a new file opens as a tab in the active group the way a file pressed
  /// in the tree does. A new folder opens nothing.
  const make = (): void => {
    const asked = naming();
    const name = typed().trim();

    if (asked === null || name === "" || making) {
      return;
    }

    making = true;
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
        making = false;
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
                  naming={naming}
                  typed={typed}
                  setTyped={setTyped}
                  said={said}
                  make={make}
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

      {/* And what a right-click on a folder row drops: the two things that can
          be made in it. The tree's only menu, and the same component the tab bar
          beside it opens its own with (ADR 0019, *The tree*).

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
        )}
      </ContextMenu>
    </div>
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
function Row(props: {
  /// What the row says, which is a repository's name for a root and an entry's
  /// own name for everything below one.
  name: JSX.Element;
  path: string;
  folder: boolean;
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
  ask: (event: MouseEvent, path: string, folder: boolean) => void;
  /// Said as any press on any row begins, with which hand made it: the menu is
  /// the mouse's alone, and `contextmenu` is the one event that cannot say.
  touched: (touch: boolean) => void;
  /// Which folder a name is being typed into, where one is — and so whether the
  /// field is drawn among *this* folder's rows.
  naming: Accessor<{ at: string; folder: boolean } | null>;
  /// What has been typed into it so far, and the typing itself.
  typed: Accessor<string>;
  setTyped: Setter<string>;
  /// And what the last Enter was refused with, where it was refused.
  said: Accessor<string | null>;
  /// Enter: make it, under the name that has been typed.
  make: () => void;
  /// And Escape: take the row away and leave nothing behind.
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

  return (
    <li class={styles.row}>
      <Show
        when={props.folder}
        fallback={
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
            onContextMenu={(event) => props.ask(event, props.path, false)}
          >
            <span class={styles.name}>{props.name}</span>
          </button>
        }
      >
        <button
          type="button"
          class={styles.folder}
          style={{ "padding-left": inset() }}
          aria-expanded={props.expanded(props.path)}
          // Nothing is picked up off a folder row — there is nothing for a drop
          // to open — so this says only which hand is pressing, which is what
          // the menu below needs and the whole of what a folder's press gives
          // the pointer.
          onPointerDown={(event) =>
            props.touched(event.pointerType !== "mouse")
          }
          onClick={() => props.toggle(props.path)}
          onContextMenu={(event) => props.ask(event, props.path, true)}
        >
          <Icon
            of={props.expanded(props.path) ? faChevronDown : faChevronRight}
            class={styles.caret}
          />
          <span class={styles.name}>{props.name}</span>
        </button>

        {/* And the row a name is typed into, where this is the folder it is
            being made in: under the row pressed and above what the folder
            holds, which is where the row it becomes will be drawn. */}
        <Show when={props.naming()?.at === props.path}>
          <ul class={styles.rows}>
            <li class={styles.row}>
              <Naming
                folder={props.naming()!.folder}
                inset={`calc(${inset()} + 0.75rem)`}
                typed={props.typed}
                setTyped={props.setTyped}
                said={props.said}
                make={props.make}
                leave={props.leave}
              />
            </li>
          </ul>
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
                        depth={props.depth + 1}
                        expanded={props.expanded}
                        held={props.held}
                        toggle={props.toggle}
                        open={props.open}
                        pick={props.pick}
                        carried={props.carried}
                        ask={props.ask}
                        touched={props.touched}
                        naming={props.naming}
                        typed={props.typed}
                        setTyped={props.setTyped}
                        said={props.said}
                        make={props.make}
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

/// The row a new file or a new folder is named in: a field, and the sentence a
/// refusal came back with.
///
/// **A field on a row rather than a modal** — what is being named is a row in the
/// tree, and the tree is where it is seen (ADR 0019, *The tree*). So it is drawn
/// where the row it becomes will be, indented one level in from the folder it is
/// going into.
///
/// Enter makes it and Escape takes it away, and nothing else does either: a blur
/// leaves the field standing, because the sentence a refusal draws is beside it
/// and a field that went when the eye moved would take the sentence with it.
///
/// The text is the tree's rather than this field's, so that a refusal keeps it —
/// see the signal in [`Tree`], where the reason is.
function Naming(props: {
  /// Which of the two is being made, which is the whole of what the field says
  /// about itself: what is read aloud, and what a human sees as the placeholder.
  folder: boolean;
  /// How far in it stands, which is the row it will become's own indent.
  inset: string;
  typed: Accessor<string>;
  setTyped: Setter<string>;
  said: Accessor<string | null>;
  make: () => void;
  leave: () => void;
}): JSX.Element {
  const called = (): string =>
    props.folder ? "New folder name" : "New file name";

  return (
    <div class={styles.naming} style={{ "padding-left": props.inset }}>
      <input
        class={styles.field}
        type="text"
        value={props.typed()}
        aria-label={called()}
        placeholder={called()}
        // The field is what the press asked for, so it takes the keyboard on the
        // way in: a row that appeared and then waited to be pressed would be two
        // presses for one gesture.
        ref={(field) => queueMicrotask(() => field.focus())}
        onInput={(event) => props.setTyped(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            props.make();
          }

          // Kept off the pane behind it: Escape over a tree is this row going,
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
