//! **Quick open**: the palette Ctrl+P drops over the Code pane, matching file
//! names across every root the Conversation has.
//!
//! **What it is for is the file whose path nobody remembers**
//! ([ADR 0019](../../../docs/adr/0019-the-code-pane.md), *The tree*). The tree
//! beside it is how a file is found by where it sits; this is how one is found
//! by what it is called, which is the other half of how anybody works in a
//! checkout — and the reason find-in-files is not missed yet, a terminal tab
//! and `grep` being what covers the rest.
//!
//! **A field and a list of matches, and nothing else.** The palette is a card
//! over the page — the app's one modal, which is where Escape, the backdrop,
//! the top layer and the focus coming back all are — with the rows drawn out of
//! the same paint the app's own dropdowns use. The arrows walk the rows, Enter
//! opens the one under the walk, and Escape leaves nothing behind: no tab, no
//! state on the pane, and the focus back where the press was made.
//!
//! **The list is read when the palette opens**, once, and every keystroke after
//! that is matched on the page — see [`./matching`]. There is no watcher until
//! the stage after this one, so a list read and kept would go stale the first
//! time the agent wrote anything; and a request per keystroke would be a round
//! trip inside the gap between two letters. This component is mounted only
//! while the palette is up, which is what makes those two the same thing.
//!
//! **Every root at once, each row saying which it is in.** Two roots can hold
//! the same path — a `README.md` apiece is the ordinary case — so the Repo is
//! part of the row rather than something to pick first. And a root whose list
//! the server cut short says so under the field: a palette quietly matching
//! over half a checkout is a palette that says a file is not there.
//!
//! **What Enter opens goes into the active group**, which is the group last
//! pressed into: the press was made on a palette over the whole pane rather
//! than in any group, and where somebody was last working is where they meant
//! it. Which is the tree's own rule for a file pressed in it, said again here
//! — the opening itself is the pane's, in `Code.tsx`.

import {
  For,
  Match,
  Switch,
  createMemo,
  createSignal,
  createUniqueId,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import { useQueryClient } from "@tanstack/solid-query";

import { Modal } from "../Modal";
import { listFiles } from "../api/client";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { MOST, matching, type Found } from "./matching";
import styles from "./Quick.module.css";
import chrome from "../picking.module.css";

/// What the palette says where the Conversation has no Worktrees at all.
///
/// A Conversation before its grilling has cut one, and one that has been closed
/// and had them taken back — the tree's own sentence about the same nothing,
/// said about searching rather than about drawing rows.
export const NOTHING_TO_SEARCH =
  "This conversation has no worktree yet, so there are no files to search.";

/// And where the roots are there and not one of them listed a file.
///
/// Which is one thing rather than several: git is what answers with a root's
/// files, so a root that lists nothing is a checkout git would not answer about
/// — no git on the machine, a Worktree that has gone, a directory that is not a
/// repository. An empty checkout is the same sentence and is true of it.
export const NOTHING_LISTED =
  "Nothing in this conversation's worktrees could be listed.";

/// And where there are files and none of them match.
///
/// The one empty state that is about what was typed rather than about what the
/// Conversation has, which is why it says so: there is nothing to do about it
/// but type something else.
export const NOTHING_MATCHES = "Nothing here matches that.";

/// And what it says under the field about a root the server cut short.
///
/// Without a number in it, because the number is the server's: what the human
/// has to know is that this root is not all here, and that a file the palette
/// does not offer may still be in the checkout.
export function onlyPartOf(repo: string): string {
  return `Only part of ${repo} is here: it holds more files than quick open matches over.`;
}

/// The palette, while it is up.
///
/// Mounted by the pane only while it is open — see `Code.tsx` — which is what
/// makes the read below one per opening rather than one per pane.
export function Quick(props: {
  conversation: number;

  /// What Enter, or a press on a row, opens: the path, handed to the groups of
  /// tabs, where it opens in the active one. The tree's own prop, and the same
  /// opening behind it.
  open: (path: string) => void;

  /// And the way out, which is every way out: Escape, a press on the backdrop,
  /// and the palette closing itself behind a file it has just opened.
  close: () => void;
}): JSX.Element {
  /// Every root's files, read now.
  ///
  /// Static, because a list of files is not a thing a Nudge is about and this
  /// palette does not outlive the reading: what makes it fresh is that it is
  /// read when the palette opens, and the palette is mounted when it opens.
  ///
  /// And dropped as the palette goes, for the terminals' reason in `Code.tsx`:
  /// static means static, so an answer left in the cache would be handed
  /// straight back to the next opening — which is the stale list this is
  /// written to avoid.
  const held = useQueryClient();
  const holding = () => ["file-list", props.conversation];

  const files = useReading(() => ({
    queryKey: holding(),
    queryFn: () => listFiles(props.conversation),
    freshness: "static",
    gcTime: 0,
  }));

  onCleanup(() => held.removeQueries({ queryKey: holding(), exact: true }));

  /// What has been typed, which is the whole of what the palette holds.
  const [typed, setTyped] = createSignal("");

  /// And which row the keyboard is on: an index into the rows as they stand.
  const [walked, setWalked] = createSignal(0);

  /// The matches, best first — and only as many as are drawn.
  ///
  /// A memo, because it is read three times per keystroke — the rows, the row
  /// the keyboard is on, and what Enter opens — and matching every file of a
  /// checkout three times over is three times the work for one answer.
  const rows = createMemo<Found[]>(() =>
    matching(files.data?.roots ?? [], typed()).slice(0, MOST),
  );

  /// The same index held inside the list as it stands now.
  ///
  /// The list is rebuilt under the walk by every keystroke, so a walk left past
  /// the end would name a row that is not there — which is a screen reader sent
  /// nowhere and an Enter that opens nothing. Clamped on the way out, the way
  /// the app's own dropdowns clamp theirs.
  const walking = (): number => Math.min(walked(), Math.max(0, rows().length - 1));

  /// The roots whose lists were cut short, by the Repo they are checkouts of.
  const cut = (): string[] =>
    (files.data?.roots ?? []).filter((root) => root.cut).map((root) => root.repo);

  // The rows' own ids, for the `aria-activedescendant` that says which one the
  // keyboard is on — the listbox's own contract, borrowed here with its paint.
  // And the field itself, so that the hands are in it the moment the card
  // opens: a palette is typed into rather than pressed. `autofocus` is the same
  // thing said to the browser, which is what moves the focus as `showModal`
  // opens the dialog; this is what says it wherever that is not done for us.
  let asked!: HTMLInputElement;

  onMount(() => asked.focus());

  const list = createUniqueId();
  const rowId = (index: number): string => `${list}-${index}`;
  const field = createUniqueId();

  /// Open the row the keyboard is on, and take the palette away.
  ///
  /// The closing is this side's rather than the pane's, because the palette is
  /// finished either way: what it was for has happened.
  const take = (row: Found | undefined): void => {
    if (row === undefined) {
      return;
    }

    props.open(row.path);
    props.close();
  };

  /// The keyboard, on the field — which is where it is, the rows being
  /// something no browser will focus.
  ///
  /// Escape is not here: the dialog answers it itself and says so through
  /// `close`, which is the whole of why this is drawn in the app's modal.
  const key = (event: KeyboardEvent): void => {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setWalked(Math.min(walking() + 1, rows().length - 1));
        return;

      case "ArrowUp":
        event.preventDefault();
        setWalked(Math.max(walking() - 1, 0));
        return;

      case "Enter":
        event.preventDefault();
        take(rows()[walking()]);
        return;

      default:
        return;
    }
  };

  return (
    <Modal class={styles.palette!} open close={props.close} name="Quick open">
      <label class={styles.label} for={field}>
        Open a file
      </label>

      {/* The field, which has the focus from the moment the card opens: a
          palette is typed into rather than pressed, and `showModal` puts the
          focus on the first thing in the card that takes it. */}
      <input
        id={field}
        class={styles.field}
        type="text"
        inputmode="url"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck={false}
        placeholder="Part of a path"
        value={typed()}
        role="combobox"
        // Said off the rows rather than written down: what a palette with nothing
        // to offer has is a sentence where the list would be, and a control
        // pointing at a list that is not on the page is a screen reader sent
        // nowhere.
        aria-expanded={rows().length > 0 ? "true" : "false"}
        aria-controls={rows().length > 0 ? list : undefined}
        aria-activedescendant={rows().length > 0 ? rowId(walking()) : undefined}
        aria-autocomplete="list"
        autofocus
        ref={asked}
        onInput={(event) => {
          setWalked(0);
          setTyped(event.currentTarget.value);
        }}
        onKeyDown={key}
      />

      {/* And what is not all here, where a root was cut short. Under the field
          rather than in the rows, because it is true of the list rather than of
          anything in it. */}
      <For each={cut()}>
        {(repo) => <p class={styles.cut}>{onlyPartOf(repo)}</p>}
      </For>

      <Switch
        fallback={<Empty>Reading this conversation's files…</Empty>}
      >
        <Match when={files.isError}>
          <ErrorLine>
            Could not read this conversation's files: {files.error?.message}
          </ErrorLine>
        </Match>

        <Match when={files.data && rows().length > 0}>
          <div class={styles.rows} id={list} role="listbox">
            <For each={rows()}>
              {(row, index) => (
                <div
                  id={rowId(index())}
                  class={[
                    chrome.row,
                    styles.row,
                    index() === walking() ? chrome.walked : undefined,
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  role="option"
                  aria-selected={index() === walking() ? "true" : "false"}
                  onClick={() => take(row)}
                >
                  {/* What the file is called, and where it is under its root —
                      the name first and at the size the card is read at,
                      because the name is what was typed and the path is where
                      it turned out to live. */}
                  <span class={styles.where}>
                    <span class={styles.name}>{named(row.under)}</span>
                    <span class={styles.under}>{row.under}</span>
                  </span>

                  {/* And which root it is in, at the far end: two roots can
                      hold the same path, and this is the whole of what tells
                      the two rows apart. */}
                  <span class={styles.repo}>{row.repo}</span>
                </div>
              )}
            </For>
          </div>
        </Match>

        <Match when={files.data && files.data.roots.length === 0}>
          <Empty>{NOTHING_TO_SEARCH}</Empty>
        </Match>

        {/* Roots, and not a file among them: git is what answers with a root's
            files, so this is every root being one git would not answer about —
            which is a different thing to be told from a search that matched
            nothing. */}
        <Match
          when={files.data?.roots.every((root) => root.files.length === 0)}
        >
          <Empty>{NOTHING_LISTED}</Empty>
        </Match>

        <Match when={files.data}>
          <Empty>{NOTHING_MATCHES}</Empty>
        </Match>
      </Switch>
    </Modal>
  );
}

/// What a row is called: the last segment of the path under its root.
///
/// Cut rather than split, and on either separator: a Worktree on Windows is
/// spelled with a `\`, and what is drawn is the path as the server handed it
/// over.
function named(under: string): string {
  return under.slice(
    Math.max(under.lastIndexOf("/"), under.lastIndexOf("\\")) + 1,
  );
}
