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
//! **What git ignores is not in it**, and neither is `.git`. Both are the
//! server's doing — git is asked rather than reimplemented — so what arrives
//! here is already a listing with no `target/` in it.
//!
//! **A file is a row and not yet a press.** What a press on one opens is
//! Monaco, which is the task after this one; until then the tree draws what is
//! there and the tabs beside it hold terminals.
//!
//! No row menu, no quick open and no git status marks: those are stages 02 and
//! 03 of the roadmap, and each of them wants this tree to be here first.

import {
  faChevronDown,
  faChevronRight,
} from "@fortawesome/free-solid-svg-icons";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

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

export function Tree(props: { conversation: number }): JSX.Element {
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

  /// Which folders are open, and what each of them last read.
  ///
  /// A path is in here or it is not, and that is the whole of what open means:
  /// collapsing one takes its entry away, so expanding it again is a fresh
  /// reading rather than a look at what was drawn before.
  const [held, setHeld] = createSignal<Record<string, FolderListing>>({});

  /// And which are open with nothing read back yet, which is the moment between
  /// the press and the answer.
  const [reading, setReading] = createSignal<string[]>([]);

  const open = (path: string): boolean =>
    held()[path] !== undefined || reading().includes(path);

  /// Open a folder, or shut it.
  ///
  /// Shutting forgets what it held. Opening asks the server afresh — every
  /// time, because nothing here is told when the disk moves and the folder may
  /// have been rewritten by the agent since it was last drawn.
  const toggle = (path: string): void => {
    if (open(path)) {
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
                  open={open}
                  held={held}
                  toggle={toggle}
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
/// of one takes no typing (which is the task after this one), and a human who
/// finds that out by typing has been told too late.
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
/// A folder is a button, because pressing it does something. A file is not, yet:
/// what a press on one opens is Monaco, and this stage draws the tree the tabs
/// beside it will open into.
function Row(props: {
  /// What the row says, which is a repository's name for a root and an entry's
  /// own name for everything below one.
  name: JSX.Element;
  path: string;
  folder: boolean;
  /// How deep it is, which is the whole of what indents it.
  depth: number;
  open: (path: string) => boolean;
  held: () => Record<string, FolderListing>;
  toggle: (path: string) => void;
}): JSX.Element {
  /// The indent, in the one unit a tree has: a level.
  const inset = (): string => `${0.5 + props.depth * 0.75}rem`;

  const listing = (): FolderListing | undefined => props.held()[props.path];

  return (
    <li class={styles.row}>
      <Show
        when={props.folder}
        fallback={
          <span class={styles.file} style={{ "padding-left": inset() }}>
            <span class={styles.name}>{props.name}</span>
          </span>
        }
      >
        <button
          type="button"
          class={styles.folder}
          style={{ "padding-left": inset() }}
          aria-expanded={props.open(props.path)}
          onClick={() => props.toggle(props.path)}
        >
          <Icon
            of={props.open(props.path) ? faChevronDown : faChevronRight}
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
                        open={props.open}
                        held={props.held}
                        toggle={props.toggle}
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
