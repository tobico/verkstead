//! The paths on the settings page: the directories Verkstead may be pointed at,
//! and the extra directories every sandbox is given.
//!
//! Both are said in two places — the installation's flags or environment, and
//! `config.yaml` — and Verkstead goes by the union of the two. So both are drawn
//! here, and each row says which of the two said it: the installation's are the
//! unit's word and there is nothing on a phone that could rewrite a unit, so
//! they are read-only wherever they appear, and only the settings' own are added
//! and taken away.
//!
//! What makes this section worth having at all is the standalone install: a bare
//! binary comes up with nothing configured anywhere, admits nothing, and is
//! pointed at its first directory from here. That is the state the pane opens in
//! on a fresh machine, and it says what it costs — nothing can be registered
//! until a Watched Path exists — because a page that drew two empty lists would
//! be a page that looked finished.
//!
//! Every row reports whether the server can currently see what it names, which
//! is the one thing a human cannot check from a phone. A directory nobody has
//! made, a path typed with a letter missing, and a directory that is there but
//! outside the namespace a hardened unit can see all look the same in a text
//! field, and all three are an entry that does nothing. The server works out
//! which it is and says so in words — see `crate::paths` — and on a nix install
//! that sentence is how somebody learns the installer has to widen the unit
//! before what they saved can work.
//!
//! Only the global binds are here. A bind scoped to one Repo belongs on that
//! Repo's own pane, and its rows are not drawn in this list — but they still
//! ride along on every save this pane makes, because one request writes the
//! whole of `config.yaml` and a list sent short is a list emptied.
//!
//! Two halves in two panes, like every other section: a card in the middle pane
//! saying how the two lists stand and whether anything is wrong with them, and
//! the editing in the details pane it opens, at `/settings/paths`. Both read the
//! one settings query the sections above them read.
//!
//! A row saves on its own press. Adding one is the Add beside the field and
//! taking one away is the Remove on the row, and each is a save of the whole
//! file with the rest of it riding along as it stands: there is nothing to
//! commit afterwards, and — the point of doing it this way — the answer to the
//! save is what says whether the new entry resolves. Only the server can know
//! that, so a row typed and left uncommitted could never report it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch as Choose,
  createSignal,
  type JSX,
} from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
import { loadSettings, saveSettings } from "../api/client";
import type {
  BindEntry,
  PathSource,
  PathsView,
  Resolution,
  SettingsSaved,
  SettingsView,
  WatchedPathEntry,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { heldPaths } from "./held";
import styles from "./Paths.module.css";

/// The settings as they stand, read once for the two panes that draw them — the
/// same read, by the same key, that every other section of this page makes.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// One entry of either list as this pane draws it: what the server said about
/// it, and where it stands among the settings' *own* entries of that list.
///
/// The second is what a Remove sends, and it is not where the row stands on the
/// page: the installation's entries are interleaved with them, and a Repo's own
/// binds sit in the file between binds that are not drawn here at all. Counted
/// rather than looked up by path, because nothing stops a file naming the same
/// directory twice and a removal that took both would be taking away something
/// nobody pressed. `-1` for a row the settings do not own, which is a row with
/// no Remove on it.
type Row<Entry> = { entry: Entry; held: number };

/// The entries of one list, each told where it stands among the settings' own.
function rowed<Entry extends { source: PathSource }>(
  entries: Entry[],
): Row<Entry>[] {
  let held = 0;

  return entries.map((entry) => ({
    entry,
    held: entry.source === "Settings" ? held++ : -1,
  }));
}

/// The binds every sandbox gets, which are the ones this pane edits.
///
/// A bind scoped to a Repo is that Repo's pane's, so it is not a row here. An
/// entry nothing could be read out of comes back scoped to nothing, which is
/// why it lands in this list: it is a row somebody has to be able to correct.
function global(paths: PathsView | undefined): Row<BindEntry>[] {
  return rowed(paths?.binds ?? []).filter((row) => row.entry.repo === null);
}

/// Why the server cannot see what an entry names, or `null` where it can.
function unresolved(resolution: Resolution): string | null {
  return resolution === "Resolves" ? null : resolution.Unresolved.why;
}

/// How many of the rows drawn on this page name something the server cannot
/// currently see — whoever said them, because the installation's own go stale
/// the same way a settings row does.
function unseen(paths: PathsView | undefined): number {
  const rows = [...(paths?.watched ?? []), ...global(paths).map((r) => r.entry)];

  return rows.filter((entry) => unresolved(entry.resolution)).length;
}

/// A count with the word it counts, so that a line reads as English rather than
/// as `1 paths`.
function counted(many: number, one: string, more: string): string {
  return `${many} ${many === 1 ? one : more}`;
}

/// The paths as they stand, as the card that opens them.
///
/// What is on the card is what somebody scanning the page is after: how much of
/// each list stands, and whether anything about it wants doing — which is either
/// no Watched Path at all, or an entry that is saved and does nothing.
export function PathsCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const settings = useSettings();

  return (
    <Choose>
      <Match when={settings.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={settings.isError}>
        <ErrorLine>
          Could not read the settings: {settings.error?.message}
        </ErrorLine>
      </Match>
      <Match when={settings.data?.paths}>
        {(paths) => (
          <CardButton
            as="article"
            class={styles.pathsCard}
            open={props.open}
            press={props.press}
          >
            <h2>Paths</h2>

            {/* The state a fresh standalone install opens in, and what it costs
                said with it: a boundary around nothing admits nothing, so there
                is no repo to register and nothing to start work on. */}
            <Show when={paths().watched.length === 0}>
              <p class={styles.warning}>
                No watched path is configured, so no repo can be registered —
                Verkstead touches nothing on disk until one is.
              </p>
            </Show>

            {/* And the other thing the browser can see and the human cannot: a
                row that is saved, is in the file, and does nothing, because what
                it names is not where the server is looking. */}
            <Show when={unseen(paths()) > 0}>
              <p class={styles.warning}>
                {counted(unseen(paths()), "entry", "entries")} the server cannot
                see. Open this section to read why.
              </p>
            </Show>

            <p class={styles.standing}>
              {counted(paths().watched.length, "watched path", "watched paths")}
              , and {counted(global(paths()).length, "bind", "binds")} every
              sandbox gets.
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the two lists themselves, which is the details pane the card opens.
///
/// There is no Save over the whole of it and no Cancel: each row is its own
/// press, and a details pane is left by opening something else or by the way
/// back a narrow window draws.
export function PathsPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  const told = (): SettingsView | undefined => settings.data;

  /// The settings' own entries of both lists, as the strings a save sends back
  /// — the same ones every other section of this page rides along untouched.
  const held = () => heldPaths(told());

  const save = useMutation(() => ({
    mutationFn: (lists: {
      watched_paths: string[];
      sandbox_binds: string[];
    }) => {
      const standing = told();

      return saveSettings({
        // The rest of both files as they stand: the endpoint writes them whole,
        // and this pane has no business with any of it.
        git_author: standing?.git_author ?? { name: "", email: "" },
        github_token: "Keep",
        rust_build_cache: {
          enabled: standing?.rust_build_cache.enabled ?? true,
          size: standing?.rust_build_cache.size_configured
            ? (standing?.rust_build_cache.size ?? "")
            : "",
        },
        share_viewer_url: standing?.share_viewer_url ?? "",
        ...lists,
      });
    },
    onSuccess: (saved: SettingsSaved) => {
      // The save's answer *is* a fresh read of both files, and it is the only
      // thing that knows whether what was just added resolves.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  /// One list rewritten and the other ridden along as it stands, which is every
  /// press on this pane.
  const writeWatched = (watched_paths: string[]) =>
    save.mutate({ watched_paths, sandbox_binds: held().sandbox_binds });

  const writeBinds = (sandbox_binds: string[]) =>
    save.mutate({ watched_paths: held().watched_paths, sandbox_binds });

  /// A list with the entry standing at `at` taken out of it.
  const without = (entries: string[], at: number): string[] =>
    entries.filter((_, which) => which !== at);

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title="Paths" />
      </PaneSticky>

      <Choose>
        <Match when={settings.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            Could not read the settings: {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={told()?.paths}>
          {(paths) => (
            <div class={styles.paths}>
              <section class={styles.list}>
                <h2>Watched paths</h2>

                <Note>
                  The directories Verkstead may operate inside. A repo is
                  registered only from within one, and nothing outside every
                  watched path is touched.
                </Note>

                {/* What a fresh standalone install opens on, said with what it
                    costs: an empty list on its own would read as a page with
                    nothing left to ask for. */}
                <Show when={paths().watched.length === 0}>
                  <p class={styles.warning}>
                    No watched path is configured anywhere, so nothing can be
                    registered. Add the directory your repositories are in.
                  </p>
                </Show>

                <Rows
                  rows={rowed(paths().watched)}
                  none="No watched paths."
                  saving={save.isPending}
                  remove={(at) =>
                    writeWatched(without(held().watched_paths, at))
                  }
                />

                <Adding
                  id="watched-path"
                  label="Add a watched path"
                  placeholder="/home/you/src"
                  saving={save.isPending}
                  add={(path) =>
                    writeWatched([...held().watched_paths, path])
                  }
                />
              </section>

              <section class={styles.list}>
                <h2>Sandbox binds</h2>

                {/* The boundary this list moves, stated beside the editor the
                    way the build cache states its own — brief, and not a
                    confirmation step: it is the human's own machine, and a
                    press they have to acknowledge twice is one they stop
                    reading. */}
                <Note>
                  Extra directories every sandboxed session may read and write,
                  over and above the worktree it works in. Each entry widens
                  what a session can reach, so add only what one needs.
                </Note>

                <Rows
                  rows={global(paths())}
                  none="No binds every sandbox gets."
                  saving={save.isPending}
                  remove={(at) => writeBinds(without(held().sandbox_binds, at))}
                />

                <Adding
                  id="sandbox-bind"
                  label="Add a bind"
                  placeholder="/var/cache/something"
                  saving={save.isPending}
                  add={(path) => writeBinds([...held().sandbox_binds, path])}
                />
              </section>

              <Show when={save.isError}>
                <ErrorLine class={styles.failure}>
                  The settings could not be saved: {save.error?.message}
                </ErrorLine>
              </Show>
            </div>
          )}
        </Match>
      </Choose>
    </>
  );
}

/// One list's rows, or the line that says it has none.
function Rows(props: {
  rows: Row<WatchedPathEntry | BindEntry>[];
  /// What is said where the list is empty.
  none: string;
  /// Whether a save is in flight, which is what stops a second press landing on
  /// a list the first one is still rewriting.
  saving: boolean;
  /// Take one away, by where it stands among the settings' own.
  remove: (held: number) => void;
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
              </div>

              {/* The one thing a human cannot check from a phone, in the
                  server's own words. */}
              <Show when={unresolved(row.entry.resolution)}>
                {(why) => <p class={styles.unresolved}>{why()}</p>}
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
/// Typed rather than picked out of a browser, for the reason the Repos' form is
/// typed: nothing here scans a filesystem to offer choices from it, and what the
/// server makes of the path is the only thing that decides what it does.
function Adding(props: {
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
        <input
          id={props.id}
          type="text"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          placeholder={props.placeholder}
          value={typed()}
          onInput={(ev) => setTyped(ev.currentTarget.value)}
        />
        <button type="submit" disabled={props.saving}>
          Add
        </button>
      </div>
    </form>
  );
}
