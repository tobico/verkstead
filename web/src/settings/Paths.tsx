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
//! Repo's own pane — see `repos/RepoBinds.tsx`, which draws the same rows out of
//! the same read — and its rows are not drawn in this list. They still ride
//! along on every save this pane makes, because one request writes the whole of
//! `config.yaml` and a list sent short is a list emptied.
//!
//! Two halves in two panes, like every other section: a card in the middle pane
//! saying how the two lists stand and whether anything is wrong with them, and
//! the editing in the details pane it opens, at `/settings/paths`. Both read the
//! one settings query the sections above them read.
//!
//! A row saves on its own press. Adding one is the Add beside the field and
//! taking one away is the Remove on the row, and each is a save of the whole
//! file with the rest of it riding along as it stands — see `PathEditor.tsx`,
//! which is what both places paths are edited make that save with.

import { Match, Show, Switch as Choose, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import type { BindEntry, PathsView } from "../api/types";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import {
  Adding,
  Rows,
  type Row,
  rowed,
  unresolved,
  useSettings,
  useWritingPaths,
  without,
} from "./PathEditor";
import styles from "./Paths.module.css";

/// The binds every sandbox gets, which are the ones this pane edits.
///
/// A bind scoped to a Repo is that Repo's pane's, so it is not a row here. An
/// entry nothing could be read out of comes back scoped to nothing, which is
/// why it lands in this list: it is a row somebody has to be able to correct.
function global(paths: PathsView | undefined): Row<BindEntry>[] {
  return rowed(paths?.binds ?? []).filter((row) => row.entry.repo === null);
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
  const { settings, told, held, save, writeWatched, writeBinds } =
    useWritingPaths();

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
