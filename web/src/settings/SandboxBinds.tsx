//! The sandbox binds on the settings page: the extra paths every sandbox is
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
//! account without anything being said here, and a path is what somebody adds
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
//! The card counts the ones the server cannot see. A path that has quietly
//! stopped resolving is exactly what nobody goes looking for, so the one warning
//! there is has to be where somebody scanning the settings will meet it.
//!
//! Every bind is here, because every bind is a path each sandbox gets. A bind
//! could once be written for one Repo — `name=path` — and that grammar is gone:
//! an entry still in it reaches no session, draws no row, and is dropped from
//! the file by the next save this pane makes.
//!
//! Two halves in two panes, like every other section: a card in the middle pane
//! saying how the list stands and whether anything is wrong with it, and the
//! editing in the details pane it opens, at `/settings/sandbox-binds`. Both read
//! the one settings query the sections above them read.
//!
//! The pane is the list and nothing else: one line saying what the section is
//! for, the rows, and the field that adds another. There is no heading inside it
//! — the pane's own title says what these are, and a section with one subsection
//! was saying it twice.
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
import styles from "./SandboxBinds.module.css";

/// The binds, as the rows this pane draws — every one of them, whichever of the
/// two sources said it.
///
/// An entry nothing could be read out of is one of them: it is a row somebody
/// has to be able to correct.
function drawn(paths: PathsView | undefined): Row<BindEntry>[] {
  return rowed(paths?.binds ?? []);
}

/// How many entries the list holds that name something the server cannot
/// currently see — whoever said them, because the installation's own go stale
/// the same way a settings row does.
function unseen(paths: PathsView | undefined): number {
  return (paths?.binds ?? []).filter((entry) => unresolved(entry.resolution))
    .length;
}

/// What the card says about them: how many, and where to go and read why.
function unseenSays(paths: PathsView | undefined): string {
  const many = counted(unseen(paths), "entry", "entries");

  return `${many} the server cannot see. Open this section to read why.`;
}

/// A count with the word it counts, so that a line reads as English rather than
/// as `1 paths`.
function counted(many: number, one: string, more: string): string {
  return `${many} ${many === 1 ? one : more}`;
}

/// The binds as they stand, as the card that opens them.
///
/// What is on the card is what somebody scanning the page is after: how much of
/// the list stands, and whether anything about it wants doing — which is an
/// entry that is saved and does nothing.
export function SandboxBindsCard(props: {
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
            class={styles.bindsCard}
            open={props.open}
            press={props.press}
          >
            <h2>Sandbox binds</h2>

            {/* The one thing the browser can see and the human cannot: a row
                that is saved, is in the file, and does nothing, because what it
                names is not where the server is looking. */}
            <Show when={unseen(paths()) > 0}>
              <p class={styles.warning}>{unseenSays(paths())}</p>
            </Show>

            <p class={styles.standing}>
              {counted(paths().binds.length, "path", "paths")}.
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
export function SandboxBindsPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const { settings, told, held, save, writeBinds } = useWritingPaths();

  return (
    <>
      <PaneSticky>
        <PaneHead
          back={{ to: "Settings", go: props.back }}
          title="Sandbox binds"
        />
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
            <div>
              {/* What the section is for, in the one line a pane on this page
                  says its own in. No heading over it: the pane's title has said
                  what these are, and the one subsection this pane held was
                  saying it a second time. */}
              <Note>
                Configures additional paths which are accessible from within the
                sandbox.
              </Note>

              <Rows
                rows={drawn(paths())}
                none="No paths are configured yet."
                saving={save.isPending}
                remove={(at) => writeBinds(without(held().sandbox_binds, at))}
              />

              <Adding
                id="sandbox-path"
                label="Add a path"
                placeholder="/var/cache/something"
                saving={save.isPending}
                add={(path) => writeBinds([...held().sandbox_binds, path])}
              />

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
