//! The paths on the settings page: the extra directories every sandbox is
//! given, over and above the worktree a session works in.
//!
//! They are said in two places — the installation's flags or environment, and
//! `config.yaml` — and a session gets the union of the two. So both are drawn
//! here, and each row says which of the two said it: the installation's are the
//! unit's word and there is nothing on a phone that could rewrite a unit, so
//! they are read-only wherever they appear, and only the settings' own are added
//! and taken away.
//!
//! An empty list is the ordinary state rather than a machine half set up: a
//! session reaches its own worktree, its Repo's git directory and its Profile's
//! account without anything being said here, and a bind is what somebody adds
//! when one needs a package registry or a cache beyond that.
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
//! The card counts every one of them, including the ones this pane does not
//! list. A bind that has quietly stopped resolving is exactly what nobody goes
//! looking for, so the one warning there is has to be where somebody scanning
//! the settings will meet it.
//!
//! Only the global binds are here, and the strays. A bind scoped to one Repo is
//! not drawn at all: the section that drew those rows stood on that Repo's own
//! pane, which is gone, and `config.yaml` is where one is read and corrected
//! now. Those rows still ride along on every save this pane makes, because one
//! request writes the whole of `config.yaml` and a list sent short is a list
//! emptied.
//!
//! A **stray** is a bind written against a name no registered Repo has, which
//! unregistering a Repo leaves behind and a misspelled name creates outright.
//! It is drawn here where a scoped one is not, because nothing will ever be
//! given it: a row that no session gets and no page shows is one nobody can take
//! away — the same reason an entry nothing can be read out of at all is a row
//! here. So they are drawn among the global ones, each saying which name it was
//! written for and that nothing holds it.
//!
//! Two halves in two panes, like every other section: a card in the middle pane
//! saying how the list stands and whether anything is wrong with it, and the
//! editing in the details pane it opens, at `/settings/paths`. Both read the
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
import { useRepos } from "../repos/RepoList";
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

/// The binds every sandbox gets, which are the ones this pane is *about*.
///
/// A bind scoped to a Repo is not one of these. An entry nothing could be read
/// out of comes back scoped to nothing, which is why it counts as one: it is a
/// row somebody has to be able to correct.
function global(paths: PathsView | undefined): Row<BindEntry>[] {
  return rowed(paths?.binds ?? []).filter((row) => row.entry.repo === null);
}

/// And the rows this pane actually draws: those, and the strays.
///
/// A **stray** is a bind written against a name no registered Repo has, and it
/// has to be somewhere: it sits in `config.yaml`, no session will ever be given
/// it, and a row that is drawn nowhere is a row nobody can take away.
/// Unregistering a Repo leaves its binds like this, and so does a misspelled
/// name. So they land here, beside the global ones, each saying which name it
/// was written for.
///
/// `registered` is `undefined` until the Repos have been read, and nothing is
/// called a stray before then: a row that appeared and vanished as that read
/// landed would be worse than one that arrives a moment after the rest.
function drawn(
  paths: PathsView | undefined,
  registered: Set<string> | undefined,
): Row<BindEntry>[] {
  return rowed(paths?.binds ?? []).filter(
    (row) =>
      row.entry.repo === null ||
      (registered !== undefined && !registered.has(row.entry.repo)),
  );
}

/// How many entries the list holds that name something the server cannot
/// currently see — whoever said them, because the installation's own go stale
/// the same way a settings row does.
///
/// Every bind, including the ones this pane does not list: a saved entry that
/// quietly does nothing is the one thing a human cannot check from a phone, so a
/// count that skipped one would leave it with nothing saying so anywhere.
function unseen(paths: PathsView | undefined): number {
  return (paths?.binds ?? []).filter((entry) => unresolved(entry.resolution))
    .length;
}

/// What the card says about them: how many, and where to go and read why.
///
/// A row this pane does not list is counted too — see [`unseen`] — and the line
/// still sends somebody here, because this is where every row anybody can do
/// anything about stands.
function unseenSays(paths: PathsView | undefined): string {
  const many = counted(unseen(paths), "entry", "entries");

  return `${many} the server cannot see. Open this section to read why.`;
}

/// A count with the word it counts, so that a line reads as English rather than
/// as `1 paths`.
function counted(many: number, one: string, more: string): string {
  return `${many} ${many === 1 ? one : more}`;
}

/// The paths as they stand, as the card that opens them.
///
/// What is on the card is what somebody scanning the page is after: how much of
/// the list stands, and whether anything about it wants doing — which is an
/// entry that is saved and does nothing.
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

            {/* The one thing the browser can see and the human cannot: a row
                that is saved, is in the file, and does nothing, because what it
                names is not where the server is looking. Counted whether or not
                this pane lists it, a bind nothing draws going stale unwatched
                the same way one here does. */}
            <Show when={unseen(paths()) > 0}>
              <p class={styles.warning}>{unseenSays(paths())}</p>
            </Show>

            <p class={styles.standing}>
              {counted(global(paths()).length, "bind", "binds")} every sandbox
              gets.
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the list itself, which is the details pane the card opens.
///
/// There is no Save over the whole of it and no Cancel: each row is its own
/// press, and a details pane is left by opening something else or by the way
/// back a narrow window draws.
export function PathsPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const { settings, told, held, save, writeBinds } = useWritingPaths();

  // And which Repos are registered, which is the only thing that tells a bind
  // written for one from a stray — see [`drawn`]. `undefined` while the read is
  // in flight, which is not the same as none of them.
  const repos = useRepos();
  const registered = (): Set<string> | undefined =>
    repos.data && new Set(repos.data.map((repo) => repo.name));

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
                  rows={drawn(paths(), registered())}
                  none="No binds every sandbox gets."
                  saving={save.isPending}
                  remove={(at) => writeBinds(without(held().sandbox_binds, at))}
                  // Every row here that names a Repo is a stray, by the way this
                  // list is drawn — so a row that names one says which name, and
                  // says that nothing holds it.
                  stray
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
