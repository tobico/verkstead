//! One Repo's Sandbox Configuration: the binds only sessions in that repository
//! get, on the pane the repository is opened at.
//!
//! A bind is either every sandbox's or one Repo's, and the two are said in the
//! one file in the one grammar — `/abs/path` for the first and `name=/abs/path`
//! for the second, where the name is the one the Repo is registered under. So
//! the two are drawn in the two places they are *about*: the global ones are the
//! Paths section of the settings page, and a Repo's own are here, where somebody
//! reading about the repository will meet them. A list of `verkstead=…` entries
//! on a page listing every path would be a list nobody could scan.
//!
//! Which Repo an entry is for is the name rather than the path: that is what the
//! configuration says, and it is what [`crate::sandbox::SandboxConfig`] composes
//! by. Two registered Repos of one name therefore share what is written for the
//! name — a collision the human can see on both panes and rename their way out
//! of.
//!
//! And a name nothing is registered under has no pane to be drawn on, so it is
//! drawn on the Paths pane instead — see `settings/Paths.tsx`, which calls one
//! of those a stray. Unregistering a Repo makes one of every bind written for
//! it, which is why it matters: the entries stay in the file, no session is
//! given them any more, and registering the same path again brings the same Repo
//! back and the binds with it.
//!
//! The rows are the Paths pane's own, out of the same read and saved by the same
//! write — see `settings/PathEditor.tsx`. So the installation's entries draw
//! labelled and without a press, because a unit's word is not something a phone
//! can rewrite; the settings' own are added and taken away here; and every row
//! says whether the server can currently see what it names, which is the one
//! thing a human cannot check from a phone.
//!
//! Drawn whether or not the Repo has any. The pane is where somebody looks to
//! learn that a repository *can* be given a cache of its own, and a section that
//! appeared only once one existed would be a section nobody could find the first
//! time.

import { Match, Show, Switch as Choose, type JSX } from "solid-js";

import type { BindEntry } from "../api/types";
import { Empty, ErrorLine, Note } from "../notices";
import {
  Adding,
  Rows,
  type Row,
  rowed,
  useWritingPaths,
  without,
} from "../settings/PathEditor";
import styles from "./RepoBinds.module.css";

/// This Repo's own binds, told where each stands among the settings' own.
///
/// Counted over the whole list and filtered afterwards, which is the order that
/// matters: the count is a place in the file, and the file holds every Repo's
/// binds and the global ones together. Filtering first would count the rows on
/// this pane and take somebody else's entry away.
function mine(binds: BindEntry[], repo: string): Row<BindEntry>[] {
  return rowed(binds).filter((row) => row.entry.repo === repo);
}

export function RepoBinds(props: {
  /// The Repo, by the name it is registered under — which is the name a bind is
  /// written against.
  repo: string;
}): JSX.Element {
  const { settings, told, held, save, writeBinds } = useWritingPaths();

  return (
    <section class={styles.binds}>
      <h2>Sandbox configuration</h2>

      {/* What every entry here costs, said beside the editor rather than as a
          step to press through: it is the human's own machine, and a press they
          have to acknowledge twice is one they stop reading. */}
      <Note>
        Extra directories a session in this repo may read and write, over and
        above the worktree it works in. Each entry widens what those sessions
        can reach, so add only what one needs.
      </Note>

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
            <>
              <Rows
                rows={mine(paths().binds, props.repo)}
                none="No binds of its own."
                saving={save.isPending}
                remove={(at) => writeBinds(without(held().sandbox_binds, at))}
              />

              <Adding
                id="repo-bind"
                label="Add a bind for this repo"
                placeholder="/var/cache/something"
                saving={save.isPending}
                // Written against the name, which is the grammar the file holds
                // and the flag takes: what the human typed is the directory, and
                // which Repo it is for is the pane they typed it on.
                add={(path) =>
                  writeBinds([...held().sandbox_binds, `${props.repo}=${path}`])
                }
              />
            </>
          )}
        </Match>
      </Choose>

      <Show when={save.isError}>
        <ErrorLine class={styles.failure}>
          The settings could not be saved: {save.error?.message}
        </ErrorLine>
      </Show>
    </section>
  );
}
