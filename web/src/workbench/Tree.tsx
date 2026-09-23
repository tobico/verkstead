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
//! No row menu, no quick open and no git status marks: those are stages 02 and
//! 03 of the roadmap, and each of them wants this tree to be here first.

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
import { listFileRoots, listFolder } from "../api/client";
import type { FileRoot, FolderEntry, FolderListing } from "../api/types";
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

  /// Open a folder, or shut it.
  ///
  /// Shutting forgets what it held. Opening asks the server afresh — every
  /// time, because nothing here is told when the disk moves and the folder may
  /// have been rewritten by the agent since it was last drawn.
  const toggle = (path: string): void => {
    if (expanded(path)) {
      setReading((was) => was.filter((one) => one !== path));
      setHeld((was) => {
        const rest = { ...was };
        delete rest[path];
        return rest;
      });
      return;
    }

    setReading((was) => [...was, path]);

    void listFolder(props.conversation, path)
      .then((listing) => setHeld((was) => ({ ...was, [path]: listing })))
      // A request that never landed is a folder that says why it is empty, the
      // way a folder the server refused does: the sentence is the server's
      // where there is one, and this is the sentence there is instead.
      .catch((error: Error) =>
        setHeld((was) => ({
          ...was,
          [path]: { Unreadable: { why: error.message } },
        })),
      )
      .finally(() => setReading((was) => was.filter((one) => one !== path)));
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
                />
              )}
            </For>
          </ul>
        </Match>
        <Match when={roots.data}>
          <Empty>{NO_ROOTS}</Empty>
        </Match>
      </Switch>
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
            onPointerDown={(event) => props.pick(event, props.path)}
            onClick={() => props.open(props.path)}
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
          onClick={() => props.toggle(props.path)}
        >
          <Icon
            of={props.expanded(props.path) ? faChevronDown : faChevronRight}
            class={styles.caret}
          />
          <span class={styles.name}>{props.name}</span>
        </button>

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
