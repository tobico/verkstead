//! The sandbox binds — the **Sandbox binds** part of the settings page's Sandbox
//! section: the extra paths every sandbox is given, over and above the worktree
//! a session works in.
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
//! The Sandbox card counts the ones the server cannot see — [`unseen`], which this
//! file hands it. A path that has quietly
//! stopped resolving is exactly what nobody goes looking for, so the one warning
//! there is has to be where somebody scanning the settings will meet it.
//!
//! Every bind is here, because every bind is a path each sandbox gets. A bind
//! could once be written for one Repo — `name=path` — and that grammar is gone:
//! an entry still in it reaches no session, draws no row, and is dropped from
//! the file by the next save this part makes.
//!
//! A part rather than a section of its own: the card is the Sandbox's, and the
//! pane it opens at `/settings/sandbox` holds this under the build caches — see
//! `Sandbox.tsx`. Both read the one settings query the sections above them read.
//!
//! The part is the list and nothing else: its heading, one line saying what it
//! is for, the rows, and the field that adds another.
//!
//! A row saves on its own press. Adding one is the Add beside the field and
//! taking one away is the Remove on the row, and each is a save of the whole
//! file with the rest of it riding along as it stands — see `PathEditor.tsx`,
//! which is what both places paths are edited make that save with.

import { Match, Show, Switch as Choose, type JSX } from "solid-js";

import type { BindEntry, PathsView } from "../api/types";
import { Empty, ErrorLine, Note } from "../notices";
import {
  Adding,
  Rows,
  type Row,
  rowed,
  unresolved,
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
export function unseen(paths: PathsView | undefined): number {
  return (paths?.binds ?? []).filter((entry) => unresolved(entry.resolution))
    .length;
}

/// And the list itself: the Sandbox binds part of the Sandbox pane, under its own
/// heading — see `Sandbox.tsx`, which is the pane.
///
/// There is no Save over the whole of it and no Cancel: each row is its own
/// press, and a details pane is left by opening something else or by the way
/// back a narrow window draws.
export function SandboxBinds(): JSX.Element {
  const { settings, told, held, save, writeBinds } = useWritingPaths();

  return (
    <section>
      <h3 class={styles.heading}>Sandbox Binds</h3>

      <Choose>
        <Match when={settings.isPending}>
          <Empty>Reading the settings…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            The settings couldn't be read: {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={told()?.paths}>
          {(paths) => (
            <div>
              {/* What the section is for, in the one line a pane on this page
                  says its own in, under the heading that names these in the
                  Sandbox pane. */}
              <Note>
                These paths are bound into every session's sandbox.
              </Note>

              <Rows
                rows={drawn(paths())}
                none="No paths are bound yet. Add one below."
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
                  The settings couldn't be saved: {save.error?.message}
                </ErrorLine>
              </Show>
            </div>
          )}
        </Match>
      </Choose>
    </section>
  );
}
