//! What the two places paths are edited have in common: the read they both make,
//! the write they both make, and the rows and the field they are both drawn as.
//!
//! There are two of those places because a bind belongs where the thing it is
//! for is. The watched paths and the binds every sandbox gets are the Paths
//! section — see `Paths.tsx` — and a bind scoped to one Repo is on that Repo's
//! own pane, where somebody looking at the repository will meet it. What they
//! are editing is one file either way, so what they are editing it *with* is one
//! set of parts: two copies would be two accounts of what an entry is and two
//! places for the wording of a row to drift.
//!
//! Every row says the same three things wherever it is drawn. What it names;
//! whose it is, where that is the installation's, because a unit's word is not
//! something a phone can rewrite and a row offering to take one away would be
//! offering something the server would ignore; and whether the server can
//! currently see what it names, in the server's own words. That last is the one
//! thing a human cannot check from a phone — a directory nobody has made, a path
//! typed with a letter missing, and a directory outside what a hardened unit can
//! see all look the same in a text field, and all three are an entry that does
//! nothing.
//!
//! The field an entry is written into browses. It is the shared path field —
//! see `PathField.tsx` — so what fills it is the same dropdown wherever the
//! workbench asks for a path, and the typing it extends is untouched: a press
//! sends whatever the box holds, tapped together or typed straight in.
//!
//! And every press is a save of the whole of `config.yaml` with the rest of it
//! riding along as it stands. There is nothing to commit afterwards, and — the
//! point of doing it this way — the answer to the save is what says whether the
//! new entry resolves. Only the server can know that, so a row typed and left
//! uncommitted could never report it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createSignal, type JSX } from "solid-js";

import { PathField } from "../PathField";
import { QuietButton } from "../QuietButton";
import { loadSettings, saveSettings } from "../api/client";
import type {
  BindEntry,
  PathResolution,
  PathSource,
  SettingsSaved,
  SettingsView,
  WatchedPathEntry,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty } from "../notices";
import { heldPaths } from "./held";
import styles from "./PathEditor.module.css";

/// The settings as they stand, read once for every pane that draws them — the
/// same read, by the same key, that every other section of the settings page
/// makes.
export function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// One entry of either list as it is drawn: what the server said about it, and
/// where it stands among the settings' *own* entries of that list.
///
/// The second is what a Remove sends, and it is not where the row stands on the
/// page: the installation's entries are interleaved with them, and one pane's
/// rows sit in the file among rows that pane never draws at all — a Repo's binds
/// among the global ones, and the global ones among every Repo's. Counted rather
/// than looked up by path, because nothing stops a file naming the same
/// directory twice and a removal that took both would be taking away something
/// nobody pressed. `-1` for a row the settings do not own, which is a row with
/// no Remove on it.
export type Row<Entry> = { entry: Entry; held: number };

/// The entries of one list, each told where it stands among the settings' own.
///
/// Given the whole of a list rather than the part one pane draws: the count is a
/// place in the file, so a pane that filtered first would count its own rows and
/// remove somebody else's.
export function rowed<Entry extends { source: PathSource }>(
  entries: Entry[],
): Row<Entry>[] {
  let held = 0;

  return entries.map((entry) => ({
    entry,
    held: entry.source === "Settings" ? held++ : -1,
  }));
}

/// Why the server cannot see what an entry names, or `null` where it can.
export function unresolved(resolution: PathResolution): string | null {
  return resolution === "Resolves" ? null : resolution.Unresolved.why;
}

/// Which Repo an entry is written against, or `null` where it is written
/// against none — a bind every sandbox gets, or a watched path, which is not
/// written against a Repo at all.
export function writtenFor(
  entry: WatchedPathEntry | BindEntry,
): string | null {
  return "repo" in entry ? entry.repo : null;
}

/// A list with the entry standing at `at` taken out of it.
export function without(entries: string[], at: number): string[] {
  return entries.filter((_, which) => which !== at);
}

/// The settings read, and the one write every press on either pane makes.
///
/// One request writes the whole of `config.yaml`, so a save carries every value
/// in it and the caller rewrites the one list it is about — `writeWatched` and
/// `writeBinds` are that, each riding the other list along as it stands. The
/// author, the token, the build cache and the share-on-Done switch ride along
/// the same way: what is sent is what the file holds afterwards, so a list or a
/// value left out would be one emptied.
export function useWritingPaths() {
  const queries = useQueryClient();
  const settings = useSettings();

  const told = (): SettingsView | undefined => settings.data;

  /// The settings' own entries of both lists, as the strings a save sends back.
  const held = () => heldPaths(told());

  const save = useMutation(() => ({
    mutationFn: (lists: { watched_paths: string[]; sandbox_binds: string[] }) => {
      const standing = told();

      return saveSettings({
        // The rest of both files as they stand: the endpoint writes them whole,
        // and a paths editor has no business with any of it.
        git_author: standing?.git_author ?? { name: "", email: "" },
        github_token: "Keep",
        rust_build_cache: {
          enabled: standing?.rust_build_cache.enabled ?? true,
          size: standing?.rust_build_cache.size_configured
            ? (standing?.rust_build_cache.size ?? "")
            : "",
        },
        conflict_resolution: standing?.conflict_resolution ?? "Merge",
        share_on_done: standing?.share_on_done ?? false,
        ...lists,
        // And the ignore rules left exactly where they are. Alone among the
        // settings they travel as an action rather than a value: this form has
        // nothing to say about them, and one that spoke for them could have its
        // own save refused over a pattern it never showed anybody — see
        // [`IgnoredCommentsEdit`].
        ignored_comments: "Keep",
      });
    },
    onSuccess: (saved: SettingsSaved) => {
      // The save's answer *is* a fresh read of both files, and it is the only
      // thing that knows whether what was just added resolves.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  return {
    settings,
    told,
    held,
    save,
    writeWatched: (watched_paths: string[]) =>
      save.mutate({ watched_paths, sandbox_binds: held().sandbox_binds }),
    writeBinds: (sandbox_binds: string[]) =>
      save.mutate({ watched_paths: held().watched_paths, sandbox_binds }),
  };
}

/// One list's rows, or the line that says it has none.
export function Rows(props: {
  rows: Row<WatchedPathEntry | BindEntry>[];
  /// What is said where the list is empty.
  none: string;
  /// Whether a save is in flight, which is what stops a second press landing on
  /// a list the first one is still rewriting.
  saving: boolean;
  /// Take one away, by where it stands among the settings' own.
  remove: (held: number) => void;
  /// Whether a row naming a Repo names one nothing is registered under.
  ///
  /// Set by the pane that draws the binds every sandbox gets, where a row
  /// naming a Repo can only be a stray — a bind for a registered Repo is drawn
  /// on that Repo's own pane, and one for a name no Repo has would be drawn
  /// nowhere. A Repo's own pane leaves this alone: every row there names that
  /// Repo, and saying so on each of them would say nothing.
  stray?: boolean;
}): JSX.Element {
  return (
    <Show when={props.rows.length > 0} fallback={<Empty>{props.none}</Empty>}>
      <ul class={styles.rows}>
        <For each={props.rows}>
          {(row) => (
            <li class={styles.row}>
              <div class={styles.what}>
                <span class={styles.path}>{row.entry.path}</span>

                {/* Whose the entry is, said only on the ones nothing here can
                    change: the settings' own are the ordinary case, and a
                    label on every row would say nothing. */}
                <Show when={row.entry.source === "Installation"}>
                  <span class={styles.source}>the installation's</span>
                </Show>

                {/* And which Repo it was written for, on a row that names one
                    where every other row names none. */}
                <Show when={props.stray ? writtenFor(row.entry) : null}>
                  {(repo) => (
                    <span class={styles.source}>written for {repo()}</span>
                  )}
                </Show>
              </div>

              {/* The one thing a human cannot check from a phone, in the
                  server's own words. */}
              <Show when={unresolved(row.entry.resolution)}>
                {(why) => <p class={styles.unresolved}>{why()}</p>}
              </Show>

              {/* And the other thing it cannot do, which is reach a session: a
                  bind is composed by the name a Repo is registered under, so
                  one written for a name nothing holds is given to nobody. Said
                  beside the reason it is drawn here at all. */}
              <Show when={props.stray && writtenFor(row.entry) !== null}>
                <p class={styles.unresolved}>
                  No repo is registered under that name, so no session is given
                  it.
                </p>
              </Show>

              <Show when={row.held >= 0}>
                <QuietButton
                  class={styles.remove}
                  onClick={() => {
                    if (!props.saving) {
                      props.remove(row.held);
                    }
                  }}
                >
                  Remove
                </QuietButton>
              </Show>
            </li>
          )}
        </For>
      </ul>
    </Show>
  );
}

/// And the field another is written into, with the press that saves it.
///
/// A browsing field rather than a bare box — see `PathField.tsx`. What the form
/// is has not moved: the path is still typed, Add still sends whatever the field
/// holds, and what the server makes of that path is still the only thing that
/// decides what it does. What browsing adds is the one thing a text field never
/// had, which is a look at what is actually there — and that is exactly what
/// somebody answering the workbench from a phone cannot do for themselves.
///
/// Anywhere rather than inside the Watched Paths, for all three of them. A
/// watched path is how that boundary is *said*, so a field bounded by it could
/// only ever offer what is already watched; and a bind is a directory the
/// boundary has nothing to say about at all.
export function Adding(props: {
  /// What the field is called on the page, and what its label points at.
  id: string;
  label: string;
  placeholder: string;
  saving: boolean;
  add: (path: string) => void;
}): JSX.Element {
  const [typed, setTyped] = createSignal("");

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();

    const path = typed().trim();
    if (path === "") {
      return;
    }

    // Cleared on the press rather than on the answer: what was in the box has
    // gone to the server, and the row it becomes is what the answer draws.
    setTyped("");
    props.add(path);
  };

  return (
    <form class={styles.adding} onSubmit={commit}>
      <label for={props.id}>{props.label}</label>
      <div class={styles.field}>
        <PathField
          id={props.id}
          scope="anywhere"
          placeholder={props.placeholder}
          value={typed()}
          write={(path) => setTyped(path)}
        />
        <button type="submit" disabled={props.saving}>
          Add
        </button>
      </div>
    </form>
  );
}
